//! Tests for the Output module
//!
//! Output provides structured result types that can be rendered as either
//! human-readable text or machine-parseable JSON.

use noslop::output::{
    CheckInfo, CheckListResult, CheckMatch, CheckResult, OperationResult, OutputMode, VerifyResult,
};

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
        verified: vec![CheckMatch {
            id: "TEST-1".to_string(),
            file: "src/auth.rs".to_string(),
            scope: "*.rs".to_string(),
            message: "Check auth".to_string(),
            severity: "block".to_string(),
            verified: true,
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":true"));
    assert!(json.contains("\"files_checked\":2"));
    assert!(json.contains("src/auth.rs"));
}

#[test]
fn check_result_blocking() {
    let result = CheckResult {
        passed: false,
        files_checked: 1,
        blocking: vec![CheckMatch {
            id: "TEST-2".to_string(),
            file: "src/api.rs".to_string(),
            scope: "src/api/".to_string(),
            message: "Review API changes".to_string(),
            severity: "block".to_string(),
            verified: false,
        }],
        warnings: vec![],
        verified: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":false"));
    assert!(json.contains("\"verified\":false"));
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
            scope: "src/utils/".to_string(),
            message: "Consider reviewing utility changes".to_string(),
            severity: "warn".to_string(),
            verified: false,
        }],
        verified: vec![],
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
        scope: "*.rs".to_string(),
        message: "Test message".to_string(),
        severity: "warn".to_string(),
        verified: true,
    };

    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("\"file\":\"test.rs\""));
    assert!(json.contains("\"severity\":\"warn\""));
    assert!(json.contains("\"verified\":true"));
}

// =============================================================================
// CheckListResult Serialization Tests
// =============================================================================

#[test]
fn check_list_result_serialization() {
    let result = CheckListResult {
        checks: vec![CheckInfo {
            id: ".noslop.toml:0".to_string(),
            scope: "src/**/*.rs".to_string(),
            message: "Check Rust files".to_string(),
            severity: "block".to_string(),
            source_file: ".noslop.toml".to_string(),
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(".noslop.toml:0"));
    assert!(json.contains("src/**/*.rs"));
}

#[test]
fn check_list_empty() {
    let result = CheckListResult { checks: vec![] };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"checks\":[]"));
}

// =============================================================================
// VerifyResult Serialization Tests
// =============================================================================

#[test]
fn verify_result_serialization() {
    let result = VerifyResult {
        success: true,
        check_id: "session-check".to_string(),
        message: "Verified session handling".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("session-check"));
}

#[test]
fn verify_result_failure() {
    let result = VerifyResult {
        success: false,
        check_id: "unknown".to_string(),
        message: "Check not found".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":false"));
}

// =============================================================================
// OperationResult Serialization Tests
// =============================================================================

#[test]
fn operation_result_serialization() {
    let result = OperationResult {
        success: false,
        message: "Already initialized".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("Already initialized"));
}

#[test]
fn operation_result_success() {
    let result = OperationResult {
        success: true,
        message: "Initialized successfully".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":true"));
}
