//! Check service - orchestrates assertion checking
//!
//! This service contains the pure business logic for checking assertions
//! against staged files and attestations.

use crate::core::models::{Assertion, Attestation, Severity};

/// Result of a check operation
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Whether the check passed (no blocking unattested assertions)
    pub passed: bool,
    /// Number of files checked
    pub files_checked: usize,
    /// Blocking assertions that need attestation
    pub blocking: Vec<AssertionCheckResult>,
    /// Warning assertions (don't block but shown)
    pub warnings: Vec<AssertionCheckResult>,
    /// Assertions that have been attested
    pub attested: Vec<AssertionCheckResult>,
}

/// Result for a single assertion check
#[derive(Debug, Clone)]
pub struct AssertionCheckResult {
    /// The assertion ID
    pub id: String,
    /// The file that matched
    pub file: String,
    /// The assertion target pattern
    pub target: String,
    /// The assertion message
    pub message: String,
    /// Severity level
    pub severity: Severity,
    /// Whether this assertion was attested
    pub attested: bool,
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
            attested: Vec::new(),
        }
    }

    /// Create a result for when no assertions apply
    #[must_use]
    pub const fn no_assertions(files_checked: usize) -> Self {
        Self {
            passed: true,
            files_checked,
            blocking: Vec::new(),
            warnings: Vec::new(),
            attested: Vec::new(),
        }
    }
}

/// Check assertions against attestations
///
/// This is pure business logic with no I/O.
///
/// # Arguments
///
/// * `applicable` - List of `(assertion, matched_file)` pairs
/// * `attestations` - List of staged attestations
/// * `files_checked` - Number of files that were checked
///
/// # Returns
///
/// A `CheckResult` categorizing assertions into blocking/warnings/attested
#[must_use]
pub fn check_assertions(
    applicable: &[(Assertion, String)],
    attestations: &[Attestation],
    files_checked: usize,
) -> CheckResult {
    let mut blocking = Vec::new();
    let mut warnings = Vec::new();
    let mut attested_list = Vec::new();

    for (assertion, file) in applicable {
        let is_attested = is_assertion_attested(assertion, attestations);

        let result = AssertionCheckResult {
            id: assertion.id.clone(),
            file: file.clone(),
            target: assertion.target.clone(),
            message: assertion.message.clone(),
            severity: assertion.severity,
            attested: is_attested,
        };

        match assertion.severity {
            Severity::Block if !is_attested => {
                blocking.push(result);
            },
            Severity::Warn if !is_attested => {
                warnings.push(result);
            },
            _ if is_attested => {
                attested_list.push(result);
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
        attested: attested_list,
    }
}

/// Check if an assertion has been attested
///
/// Matching logic (priority order):
/// 1. Exact ID match
/// 2. Attestation ID contains assertion message
/// 3. Attestation ID equals assertion target
fn is_assertion_attested(assertion: &Assertion, attestations: &[Attestation]) -> bool {
    attestations.iter().any(|a| {
        a.assertion_id == assertion.id
            || a.assertion_id.contains(&assertion.message)
            || a.assertion_id == assertion.target
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_assertion(id: &str, target: &str, message: &str, severity: Severity) -> Assertion {
        Assertion::new(Some(id.to_string()), target.to_string(), message.to_string(), severity)
    }

    fn make_attestation(assertion_id: &str, message: &str) -> Attestation {
        Attestation::new(assertion_id.to_string(), message.to_string(), "human".to_string())
    }

    #[test]
    fn test_no_assertions_passes() {
        let result = check_assertions(&[], &[], 5);
        assert!(result.passed);
        assert_eq!(result.files_checked, 5);
        assert!(result.blocking.is_empty());
    }

    #[test]
    fn test_unattested_blocking_fails() {
        let assertions = vec![(
            make_assertion("AST-1", "*.rs", "Review Rust", Severity::Block),
            "src/main.rs".to_string(),
        )];
        let attestations = vec![];

        let result = check_assertions(&assertions, &attestations, 1);
        assert!(!result.passed);
        assert_eq!(result.blocking.len(), 1);
    }

    #[test]
    fn test_attested_blocking_passes() {
        let assertions = vec![(
            make_assertion("AST-1", "*.rs", "Review Rust", Severity::Block),
            "src/main.rs".to_string(),
        )];
        let attestations = vec![make_attestation("AST-1", "Reviewed")];

        let result = check_assertions(&assertions, &attestations, 1);
        assert!(result.passed);
        assert!(result.blocking.is_empty());
        assert_eq!(result.attested.len(), 1);
    }

    #[test]
    fn test_unattested_warning_passes() {
        let assertions = vec![(
            make_assertion("AST-1", "*.rs", "Review Rust", Severity::Warn),
            "src/main.rs".to_string(),
        )];
        let attestations = vec![];

        let result = check_assertions(&assertions, &attestations, 1);
        assert!(result.passed);
        assert_eq!(result.warnings.len(), 1);
    }
}
