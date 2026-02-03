//! Validate checks for staged changes

use crate::{git, noslop_file};
use noslop::core::models::Severity;
use noslop::output::{CheckMatch, CheckResult, OutputMode};
use noslop::storage;

/// Validate checks for staged changes (pre-commit hook)
pub fn check_validate(ci: bool, mode: OutputMode) -> anyhow::Result<()> {
    // Get staged files
    let staged = git::staged::get_staged_files()?;

    if staged.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: 0,
            blocking: vec![],
            warnings: vec![],
            acknowledged: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load checks from .noslop.toml files
    let applicable = noslop_file::load_checks_for_files(&staged)?;

    if applicable.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: staged.len(),
            blocking: vec![],
            warnings: vec![],
            acknowledged: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load staged acknowledgments via storage abstraction
    let store = storage::ack_store();
    let acks = store.staged()?;

    // Check for unacknowledged checks
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let mut acknowledged_list = Vec::new();

    for (check, file) in &applicable {
        let is_acknowledged = acks.iter().any(|a| {
            // Priority matching: ID first, then message, then target
            a.check_id == check.id
                || a.check_id.contains(&check.message)
                || a.check_id == check.target
        });

        let check_match = CheckMatch {
            id: check.id.clone(),
            file: file.clone(),
            target: check.target.clone(),
            message: check.message.clone(),
            severity: check.severity.to_string(),
            acknowledged: is_acknowledged,
        };

        match check.severity {
            Severity::Block if !is_acknowledged => {
                blocking.push(check_match);
            },
            Severity::Warn if !is_acknowledged => {
                warnings.push(check_match);
            },
            _ if is_acknowledged => {
                acknowledged_list.push(check_match);
            },
            _ => {},
        }
    }

    let passed = blocking.is_empty();

    let result = CheckResult {
        passed,
        files_checked: staged.len(),
        blocking,
        warnings,
        acknowledged: acknowledged_list,
    };

    result.render(mode);

    if !passed {
        if !ci {
            std::process::exit(1);
        }
        anyhow::bail!("Unacknowledged checks");
    }

    Ok(())
}
