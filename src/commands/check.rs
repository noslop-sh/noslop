//! Check for staged changes

use noslop::output::{CheckMatch, CheckResult, OutputMode};

use noslop::git;
use noslop::models::Severity;
use noslop::noslop_file;
use noslop::storage;

/// Run checks for staged changes (pre-commit hook)
pub fn check_run(ci: bool, mode: OutputMode) -> anyhow::Result<()> {
    // Get staged files
    let staged = git::staged::get_staged_files()?;

    if staged.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: 0,
            blocking: vec![],
            warnings: vec![],
            verified: vec![],
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
            verified: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load staged verifications via storage abstraction
    let store = storage::verification_store();
    let verifications = store.staged()?;

    // Check for unverified checks
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let mut verified_list = Vec::new();

    for (check, file) in &applicable {
        let is_verified = verifications.iter().any(|v| {
            // Priority matching: ID first, then message, then scope
            v.check_id == check.id
                || v.check_id.contains(&check.message)
                || v.check_id == check.scope
        });

        let check_match = CheckMatch {
            id: check.id.clone(),
            file: file.clone(),
            scope: check.scope.clone(),
            message: check.message.clone(),
            severity: check.severity.to_string(),
            verified: is_verified,
        };

        match check.severity {
            Severity::Block if !is_verified => {
                blocking.push(check_match);
            },
            Severity::Warn if !is_verified => {
                warnings.push(check_match);
            },
            _ if is_verified => {
                verified_list.push(check_match);
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
        verified: verified_list,
    };

    result.render(mode);

    if !passed {
        if !ci {
            std::process::exit(1);
        }
        anyhow::bail!("Unverified checks");
    }

    Ok(())
}
