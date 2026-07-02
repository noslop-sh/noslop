//! Validate checks for staged changes

use crate::{git, noslop_file};
use noslop::adapters::detect_actor;
use noslop::core::models::Actor;
use noslop::core::services::{CheckItemResult, check_items};
use noslop::output::{CheckMatch, CheckResult, OutputMode};
use noslop::storage;

/// Validate checks for staged changes (pre-commit hook)
///
/// Blocking checks gate agents and CI. A human committer sees the same
/// guidance but is never blocked.
pub fn check_validate(ci: bool, mode: OutputMode) -> anyhow::Result<()> {
    let actor = detect_actor();
    let enforced = ci || actor.is_gated();

    // Get staged files
    let staged = git::staged::get_staged_files()?;

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

    // Load staged acknowledgments via storage abstraction
    let store = storage::ack_store();
    let acks = store.staged()?;

    // Core service does the matching; map its result to output types
    let core_result = check_items(&applicable, &acks, staged.len());

    let result = CheckResult {
        passed: core_result.passed || !enforced,
        files_checked: core_result.files_checked,
        actor: actor.name().to_string(),
        enforced,
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
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![],
    }
    .render(mode);
}
