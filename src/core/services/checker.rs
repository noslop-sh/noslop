//! Check service - orchestrates check validation
//!
//! This service contains the pure business logic for checking checks
//! against staged files and acknowledgments.

use crate::core::models::{Acknowledgment, Check, Severity};

/// Result of a check operation
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Whether the check passed (no blocking unacknowledged checks)
    pub passed: bool,
    /// Number of files checked
    pub files_checked: usize,
    /// Blocking checks that need acknowledgment
    pub blocking: Vec<CheckItemResult>,
    /// Warning checks (don't block but shown)
    pub warnings: Vec<CheckItemResult>,
    /// Checks that have been acknowledged
    pub acknowledged: Vec<CheckItemResult>,
}

/// Result for a single check item
#[derive(Debug, Clone)]
pub struct CheckItemResult {
    /// The check ID
    pub id: String,
    /// The file that matched
    pub file: String,
    /// The check target pattern
    pub target: String,
    /// The check message
    pub message: String,
    /// Severity level
    pub severity: Severity,
    /// Whether this check was acknowledged
    pub acknowledged: bool,
}

impl CheckResult {
    /// Create a result for when there are no staged files
    #[must_use]
    pub const fn no_staged_files() -> Self {
        Self {
            passed: true,
            files_checked: 0,
            blocking: Vec::new(),
            warnings: Vec::new(),
            acknowledged: Vec::new(),
        }
    }

    /// Create a result for when no checks apply
    #[must_use]
    pub const fn no_checks(files_checked: usize) -> Self {
        Self {
            passed: true,
            files_checked,
            blocking: Vec::new(),
            warnings: Vec::new(),
            acknowledged: Vec::new(),
        }
    }
}

/// Check checks against acknowledgments
///
/// This is pure business logic with no I/O.
///
/// # Arguments
///
/// * `applicable` - List of `(check, matched_file)` pairs
/// * `acks` - List of staged acknowledgments
/// * `files_checked` - Number of files that were checked
///
/// # Returns
///
/// A `CheckResult` categorizing checks into blocking/warnings/acknowledged
#[must_use]
pub fn check_items(
    applicable: &[(Check, String)],
    acks: &[Acknowledgment],
    files_checked: usize,
) -> CheckResult {
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let mut acknowledged_list = Vec::new();

    for (check, file) in applicable {
        let is_acknowledged = is_check_acknowledged(check, acks);

        let result = CheckItemResult {
            id: check.id.clone(),
            file: file.clone(),
            target: check.target.clone(),
            message: check.message.clone(),
            severity: check.severity,
            acknowledged: is_acknowledged,
        };

        match check.severity {
            Severity::Block if !is_acknowledged => {
                blocking.push(result);
            },
            Severity::Warn if !is_acknowledged => {
                warnings.push(result);
            },
            _ if is_acknowledged => {
                acknowledged_list.push(result);
            },
            _ => {},
        }
    }

    let passed = blocking.is_empty();

    CheckResult {
        passed,
        files_checked,
        blocking,
        warnings,
        acknowledged: acknowledged_list,
    }
}

/// Check if a check has been acknowledged
///
/// Matching logic (priority order):
/// 1. Exact ID match
/// 2. Ack ID contains check message
/// 3. Ack ID equals check target
fn is_check_acknowledged(check: &Check, acks: &[Acknowledgment]) -> bool {
    acks.iter().any(|a| {
        a.check_id == check.id || a.check_id.contains(&check.message) || a.check_id == check.target
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_check(id: &str, target: &str, message: &str, severity: Severity) -> Check {
        Check::new(Some(id.to_string()), target.to_string(), message.to_string(), severity)
    }

    fn make_ack(check_id: &str, message: &str) -> Acknowledgment {
        Acknowledgment::new(check_id.to_string(), message.to_string(), "human".to_string())
    }

    #[test]
    fn test_no_checks_passes() {
        let result = check_items(&[], &[], 5);
        assert!(result.passed);
        assert_eq!(result.files_checked, 5);
        assert!(result.blocking.is_empty());
    }

    #[test]
    fn test_unacknowledged_blocking_fails() {
        let checks = vec![(
            make_check("CHK-1", "*.rs", "Review Rust", Severity::Block),
            "src/main.rs".to_string(),
        )];
        let acks = vec![];

        let result = check_items(&checks, &acks, 1);
        assert!(!result.passed);
        assert_eq!(result.blocking.len(), 1);
    }

    #[test]
    fn test_acknowledged_blocking_passes() {
        let checks = vec![(
            make_check("CHK-1", "*.rs", "Review Rust", Severity::Block),
            "src/main.rs".to_string(),
        )];
        let acks = vec![make_ack("CHK-1", "Reviewed")];

        let result = check_items(&checks, &acks, 1);
        assert!(result.passed);
        assert!(result.blocking.is_empty());
        assert_eq!(result.acknowledged.len(), 1);
    }

    #[test]
    fn test_unacknowledged_warning_passes() {
        let checks = vec![(
            make_check("CHK-1", "*.rs", "Review Rust", Severity::Warn),
            "src/main.rs".to_string(),
        )];
        let acks = vec![];

        let result = check_items(&checks, &acks, 1);
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 1);
    }
}
