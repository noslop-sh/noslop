//! Validate checks for staged changes

use crate::{git, noslop_file};
use noslop::adapters::{detect_actor, ledger, telemetry};
use noslop::core::models::{Actor, CheckFireEvent};
use noslop::core::services::{CheckItemResult, check_items};
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
        render_empty(0, &actor, enforced, mode);
        return Ok(());
    }

    // Load checks from .noslop.toml files
    let applicable = noslop_file::load_checks_for_files(&staged)?;

    if applicable.is_empty() {
        render_empty(staged.len(), &actor, enforced, mode);
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
    // distinguish self-correction from rubber-stamping (see docs/SCHEMA.md).
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

    let result = CheckResult {
        passed: core_result.passed || !enforced,
        files_checked: core_result.files_checked,
        actor: actor.name().to_string(),
        enforced,
        tree_oid,
        blocking: core_result.blocking.iter().map(to_check_match).collect(),
        warnings: core_result.warnings.iter().map(to_check_match).collect(),
        acknowledged: core_result.acknowledged.iter().map(to_check_match).collect(),
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

fn render_empty(files_checked: usize, actor: &Actor, enforced: bool, mode: OutputMode) {
    CheckResult {
        passed: true,
        files_checked,
        actor: actor.name().to_string(),
        enforced,
        tree_oid: git::staged::staged_tree_oid().ok(),
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![],
    }
    .render(mode);
}
