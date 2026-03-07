//! Tests for the Output module
//!
//! Output provides structured result types that can be rendered as either
//! human-readable text or machine-parseable JSON.

use noslop::output::{CheckMatch, CheckResult, OutputMode};

// =============================================================================
// OutputMode Tests
// =============================================================================

#[test]
fn output_mode_default() {
    assert_eq!(OutputMode::default(), OutputMode::Human);
}

// =============================================================================
// CheckResult Serialization Tests
// =============================================================================

#[test]
fn check_result_serialization() {
    let result = CheckResult {
        passed: true,
        files_checked: 2,
        blocking: vec![],
        warnings: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":true"));
    assert!(json.contains("\"files_checked\":2"));
}

#[test]
fn check_result_blocking() {
    let result = CheckResult {
        passed: false,
        files_checked: 1,
        blocking: vec![CheckMatch {
            id: "TEST-2".to_string(),
            file: "src/api.rs".to_string(),
            target: "src/api/".to_string(),
            message: "Review API changes".to_string(),
            severity: "block".to_string(),
        }],
        warnings: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":false"));
    assert!(json.contains("Review API changes"));
}

#[test]
fn check_result_with_warnings() {
    let result = CheckResult {
        passed: true,
        files_checked: 1,
        blocking: vec![],
        warnings: vec![CheckMatch {
            id: "TEST-3".to_string(),
            file: "src/utils.rs".to_string(),
            target: "src/utils/".to_string(),
            message: "Consider reviewing utility changes".to_string(),
            severity: "warn".to_string(),
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":true")); // warnings don't block
    assert!(json.contains("\"severity\":\"warn\""));
}

// =============================================================================
// CheckMatch Serialization Tests
// =============================================================================

#[test]
fn check_match_serialization() {
    let m = CheckMatch {
        id: "TEST-4".to_string(),
        file: "test.rs".to_string(),
        target: "*.rs".to_string(),
        message: "Test message".to_string(),
        severity: "warn".to_string(),
    };

    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("\"file\":\"test.rs\""));
    assert!(json.contains("\"severity\":\"warn\""));
}
