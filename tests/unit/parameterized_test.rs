//! Parameterized tests using test-case
//!
//! These tests use test-case to run the same test logic with different inputs.

use noslop::core::models::Severity;
use test_case::test_case;

// =============================================================================
// Severity Tests
// =============================================================================

#[test_case("block", Severity::Block ; "block severity")]
#[test_case("BLOCK", Severity::Block ; "uppercase block")]
#[test_case("Block", Severity::Block ; "mixed case block")]
#[test_case("warn", Severity::Warn ; "warn severity")]
#[test_case("WARN", Severity::Warn ; "uppercase warn")]
#[test_case("info", Severity::Info ; "info severity")]
fn test_severity_parsing(input: &str, expected: Severity) {
    let parsed: Severity = input.parse().unwrap();
    assert_eq!(parsed, expected);
}

#[test_case("invalid" ; "invalid severity")]
#[test_case("" ; "empty string")]
#[test_case("warning" ; "warning is not valid")]
fn test_severity_parsing_errors(input: &str) {
    let result: Result<Severity, _> = input.parse();
    assert!(result.is_err(), "Expected error for input: {input:?}");
}
