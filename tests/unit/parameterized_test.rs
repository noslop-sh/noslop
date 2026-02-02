//! Parameterized tests using test-case
//!
//! These tests use test-case to run the same test logic with different inputs.

use noslop::core::models::{Fragment, Target};
use noslop::core::services::matches_target;
use std::path::PathBuf;
use test_case::test_case;

// =============================================================================
// Target Parsing Tests
// =============================================================================

#[test_case("*.rs", true ; "star extension is glob")]
#[test_case("src/**/*.rs", true ; "double star is glob")]
#[test_case("src/*.rs", true ; "dir star is glob")]
#[test_case("src/main.rs", false ; "exact path is not glob")]
#[test_case("README.md", false ; "simple file is not glob")]
fn test_is_glob(pattern: &str, expected: bool) {
    let target = Target::parse(pattern).unwrap();
    assert_eq!(target.is_glob(), expected);
}

#[test_case("file.rs#L42", Some(Fragment::Line(42)) ; "line fragment")]
#[test_case("file.rs#L10-L20", Some(Fragment::LineRange(10, 20)) ; "line range fragment")]
#[test_case("file.rs#MyClass", Some(Fragment::Symbol("MyClass".to_string())) ; "symbol fragment")]
#[test_case("file.rs", None ; "no fragment")]
fn test_fragment_parsing(input: &str, expected: Option<Fragment>) {
    let target = Target::parse(input).unwrap();
    assert_eq!(target.fragment().cloned(), expected);
}

// =============================================================================
// Matcher Tests
// =============================================================================

fn test_match(pattern: &str, file: &str) -> bool {
    let base = PathBuf::from("/repo");
    let cwd = PathBuf::from("/repo");
    matches_target(pattern, file, &base, &cwd)
}

#[test_case("*", "any.file", true ; "wildcard matches anything")]
#[test_case("*.rs", "main.rs", true ; "extension matches")]
#[test_case("*.rs", "main.py", false ; "extension rejects different")]
#[test_case("src/*.rs", "src/main.rs", true ; "dir glob matches")]
#[test_case("src/*.rs", "src/sub/main.rs", false ; "dir glob rejects nested")]
#[test_case("src/**/*.rs", "src/main.rs", true ; "recursive matches shallow")]
#[test_case("src/**/*.rs", "src/sub/main.rs", true ; "recursive matches deep")]
#[test_case("src/**/*.rs", "tests/main.rs", false ; "recursive rejects other dir")]
#[test_case("src/main.rs", "src/main.rs", true ; "exact matches")]
#[test_case("src/main.rs", "src/lib.rs", false ; "exact rejects different")]
#[test_case("src/", "src/main.rs", true ; "prefix matches file in dir")]
#[test_case("src/", "tests/main.rs", false ; "prefix rejects other dir")]
fn test_pattern_matching(pattern: &str, file: &str, expected: bool) {
    assert_eq!(test_match(pattern, file), expected, "pattern={pattern:?} file={file:?}");
}

// =============================================================================
// Severity Tests
// =============================================================================

use noslop::core::models::Severity;

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
