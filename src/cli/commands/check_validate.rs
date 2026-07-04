//! Validate checks for staged changes

use crate::{git, noslop_file};
use noslop::adapters::remote::{FetchedCheckSet, RemoteCheckSet, load_remote_checks};
use noslop::adapters::{detect_actor, ledger, telemetry};
use noslop::core::models::{Actor, Check, CheckFireEvent, Severity};
use noslop::core::services::{CheckItemResult, check_items, matches_target, merge_checks};
use noslop::output::{CheckMatch, CheckResult, OutputMode};
use noslop::storage;

/// Validate checks for staged changes (pre-commit hook) or, with
/// `diff_base`, for everything a branch changed (CI mode).
///
/// Blocking checks gate agents and CI. A human committer sees the same
/// guidance but is never blocked.
///
/// In diff-base mode the file list is the branch diff and acknowledgments
/// come from the ledger records committed in the branch — so a commit made
/// with `--no-verify` (no ledger record) fails here.
pub fn check_validate(ci: bool, diff_base: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let actor = detect_actor();
    // Diff-base is the CI source-of-truth pass: always enforced
    let enforced = ci || diff_base.is_some() || actor.is_gated();

    // Files under scrutiny: the branch diff (CI) or the index (pre-commit)
    let staged = match diff_base {
        Some(base) => git::staged::diff_files(base)?,
        None => git::staged::get_staged_files()?,
    };

    if staged.is_empty() {
        render_empty(0, &actor, enforced, mode, None);
        return Ok(());
    }

    // Load checks from .noslop.toml files, then merge the org's cloud set
    // (fail-open: a cloud outage degrades to local checks, never a block)
    let local = noslop_file::load_checks_for_files(&staged)?;
    let fetched = load_remote_checks(&noslop_file::load_remote_config());
    let remote_set = fetched.as_ref().map(|f| &f.set);
    let (remote_gating, remote_monitor) = partition_remote(remote_set, &staged, &actor);
    let applicable = merge_checks(local, remote_gating);

    if applicable.is_empty() && remote_monitor.is_empty() {
        render_empty(staged.len(), &actor, enforced, mode, fetched.as_ref());
        return Ok(());
    }

    // Acknowledgments: committed ledger records (CI) or staged acks (local)
    let acks = if diff_base.is_some() {
        ledger::load_pending()?
    } else {
        storage::ack_store().staged()?
    };

    // Core service does the matching; map its result to output types
    let core_result = check_items(&applicable, &acks, staged.len());

    // Gate-time tree oid: joined against ledger tree oids downstream to
    // distinguish action rate from answers that change nothing (see docs/SCHEMA.md).
    let tree_oid = git::staged::staged_tree_oid().ok();

    // Telemetry: record every surfaced (unacknowledged) check for stats.
    // Best-effort — a telemetry failure must never block a commit. CI
    // diff-base runs don't log: events are per-clone developer telemetry.
    if diff_base.is_none()
        && let Some(tree_oid) = tree_oid.clone()
    {
        let events: Vec<CheckFireEvent> = core_result
            .blocking
            .iter()
            .chain(core_result.warnings.iter())
            .map(|item| {
                CheckFireEvent::new(
                    item.id.clone(),
                    item.file.clone(),
                    item.severity,
                    actor.name().to_string(),
                    tree_oid.clone(),
                )
            })
            .collect();
        let _ = telemetry::append_events(&events);
    }

    // Monitor-state cloud checks: evaluated for telemetry, never surfaced
    // to the agent and never gating (the Semgrep Monitor trial stage)
    let monitor_result = check_items(&remote_monitor, &acks, staged.len());
    let monitor: Vec<CheckMatch> = monitor_result
        .blocking
        .iter()
        .chain(monitor_result.warnings.iter())
        .map(to_check_match)
        .collect();

    let result = CheckResult {
        passed: core_result.passed || !enforced,
        files_checked: core_result.files_checked,
        actor: actor.name().to_string(),
        enforced,
        tree_oid,
        check_set_version: remote_set.map(|s| s.check_set_version.clone()),
        check_set_age_seconds: fetched.as_ref().map(|f| f.age_seconds),
        blocking: core_result.blocking.iter().map(to_check_match).collect(),
        warnings: core_result.warnings.iter().map(to_check_match).collect(),
        acknowledged: core_result.acknowledged.iter().map(to_check_match).collect(),
        monitor,
    };

    result.render(mode);

    if enforced && !core_result.passed {
        if !ci {
            std::process::exit(1);
        }
        anyhow::bail!("Unacknowledged checks");
    }

    Ok(())
}

/// Checks paired with the staged file they matched
type MatchedChecks = Vec<(Check, String)>;

/// Split the cloud set into gating pairs and silently-evaluated monitor
/// pairs, matched against the staged files. Active bypass grants exempt
/// the current actor at enforcement time — never at visibility time.
fn partition_remote(
    set: Option<&RemoteCheckSet>,
    staged: &[String],
    actor: &Actor,
) -> (MatchedChecks, MatchedChecks) {
    let Some(set) = set else {
        return (Vec::new(), Vec::new());
    };
    let Ok(cwd) = std::env::current_dir() else {
        return (Vec::new(), Vec::new());
    };
    let now = chrono::Utc::now();

    let mut gating = Vec::new();
    let mut monitor = Vec::new();
    for remote in &set.checks {
        let bypassed = remote.bypasses.iter().any(|b| b.exempts(actor.name(), &now));
        for file in staged {
            if !matches_target(&remote.target, file, &cwd, &cwd) {
                continue;
            }
            let check = Check::new(
                Some(remote.id.clone()),
                remote.target.clone(),
                remote.message.clone(),
                remote.severity.parse().unwrap_or(Severity::Block),
            );
            if remote.state == "monitor" || bypassed {
                monitor.push((check, file.clone()));
            } else {
                gating.push((check, file.clone()));
            }
        }
    }
    (gating, monitor)
}

fn to_check_match(item: &CheckItemResult) -> CheckMatch {
    CheckMatch {
        id: item.id.clone(),
        file: item.file.clone(),
        target: item.target.clone(),
        message: item.message.clone(),
        severity: item.severity.to_string(),
        acknowledged: item.acknowledged,
    }
}

fn render_empty(
    files_checked: usize,
    actor: &Actor,
    enforced: bool,
    mode: OutputMode,
    fetched: Option<&FetchedCheckSet>,
) {
    CheckResult {
        passed: true,
        files_checked,
        actor: actor.name().to_string(),
        enforced,
        tree_oid: git::staged::staged_tree_oid().ok(),
        check_set_version: fetched.map(|f| f.set.check_set_version.clone()),
        check_set_age_seconds: fetched.map(|f| f.age_seconds),
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![],
        monitor: vec![],
    }
    .render(mode);
}
