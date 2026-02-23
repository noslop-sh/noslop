//! Validate checks for staged changes

use noslop::adapters::{FileReviewStore, GitVersionControl, TomlCheckRepository};
use noslop::core::models::Severity;
use noslop::core::ports::{CheckRepository, ReviewStore, VersionControl};
use noslop::output::{CheckMatch, CheckResult, OutputMode};

/// Validate checks for staged changes (pre-commit hook)
pub fn check_validate(ci: bool, mode: OutputMode) -> anyhow::Result<()> {
    // Get staged files
    let vcs = GitVersionControl::current_dir()?;
    let staged = vcs.staged_files()?;

    if staged.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: 0,
            blocking: vec![],
            warnings: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    // Load checks from .noslop.toml files via TomlCheckRepository
    let repo = TomlCheckRepository::current_dir()?;
    let applicable = repo.find_for_files(&staged)?;

    // Also check for open review feedbacks that apply to staged files
    let review_store = FileReviewStore::new();
    let mut all_matches: Vec<(noslop::Check, String)> = applicable;

    for file in &staged {
        for review in review_store.find_blocking_for_file(file)? {
            for feedback in review.feedbacks_for_file(file) {
                if feedback.is_blocking() {
                    // Create a synthetic check to represent the blocking feedback
                    let check = noslop::Check::new(
                        &feedback.id,
                        noslop::Target::file(file),
                        &feedback.message,
                        feedback.severity,
                    );
                    all_matches.push((check, file.clone()));
                }
            }
        }
    }

    if all_matches.is_empty() {
        let result = CheckResult {
            passed: true,
            files_checked: staged.len(),
            blocking: vec![],
            warnings: vec![],
        };
        result.render(mode);
        return Ok(());
    }

    let mut blocking = Vec::new();
    let mut warnings = Vec::new();

    for (check, file) in &all_matches {
        let check_match = CheckMatch {
            id: check.id.clone(),
            file: file.clone(),
            target: check.target.path.clone(),
            message: check.message.clone(),
            severity: check.severity.to_string(),
        };

        match check.severity {
            Severity::Block => blocking.push(check_match),
            Severity::Warn => warnings.push(check_match),
            Severity::Info => {}, // Info never shown in check output
        }
    }

    let passed = blocking.is_empty();

    let result = CheckResult {
        passed,
        files_checked: staged.len(),
        blocking,
        warnings,
    };

    result.render(mode);

    if !passed {
        if !ci {
            std::process::exit(1);
        }
        anyhow::bail!("Blocking checks found");
    }

    Ok(())
}
