//! Check assertions for staged changes

use noslop::output::{AssertionMatch, CheckResult, OutputMode};

use crate::git;
use crate::models::Severity;
use crate::noslop_file;
use crate::storage;

/// Check assertions for staged changes (pre-commit hook)
pub fn check(ci: bool, mode: OutputMode) -> anyhow::Result<()> {
    // Get staged files
    let staged = git::staged::get_staged_files()?;

    if staged.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: 0,
            blocking: vec![],
            warnings: vec![],
            attested: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load assertions from .noslop.toml files
    let applicable = noslop_file::load_assertions_for_files(&staged)?;

    if applicable.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: staged.len(),
            blocking: vec![],
            warnings: vec![],
            attested: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load staged attestations via storage abstraction
    let store = storage::attestation_store();
    let attestations = store.staged()?;

    // Check for unattested assertions
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let mut attested_list = Vec::new();

    for (assertion, file) in &applicable {
        let is_attested = attestations.iter().any(|a| {
            // Priority matching: ID first, then message, then target
            a.assertion_id == assertion.id
                || a.assertion_id.contains(&assertion.message)
                || a.assertion_id == assertion.target
        });

        let assertion_match = AssertionMatch {
            id: assertion.id.clone(),
            file: file.clone(),
            target: assertion.target.clone(),
            message: assertion.message.clone(),
            severity: assertion.severity.to_string(),
            attested: is_attested,
        };

        match assertion.severity {
            Severity::Block if !is_attested => {
                blocking.push(assertion_match);
            },
            Severity::Warn if !is_attested => {
                warnings.push(assertion_match);
            },
            _ if is_attested => {
                attested_list.push(assertion_match);
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
        attested: attested_list,
    };

    result.render(mode);

    if !passed {
        if !ci {
            std::process::exit(1);
        }
        anyhow::bail!("Unattested assertions");
    }

    Ok(())
}
