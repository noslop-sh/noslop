//! Tests for the Output module
//!
//! Output provides structured result types that can be rendered as either
//! human-readable text or machine-parseable JSON.

use noslop::output::{
    AckResult, CheckInfo, CheckListResult, CheckMatch, CheckResult, ENVELOPE_SCHEMA,
    OperationResult, OutputMode, UploadEnvelope,
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
        actor: "human".to_string(),
        enforced: false,
        tree_oid: None,
        check_set_version: None,
        check_set_age_seconds: None,
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![CheckMatch {
            id: "TEST-1".to_string(),
            file: "src/auth.rs".to_string(),
            target: "*.rs".to_string(),
            message: "Check auth".to_string(),
            severity: "block".to_string(),
            acknowledged: true,
        }],
        monitor: vec![],
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
        actor: "claude-code".to_string(),
        enforced: true,
        tree_oid: None,
        check_set_version: None,
        check_set_age_seconds: None,
        blocking: vec![CheckMatch {
            id: "TEST-2".to_string(),
            file: "src/api.rs".to_string(),
            target: "src/api/".to_string(),
            message: "Review API changes".to_string(),
            severity: "block".to_string(),
            acknowledged: false,
        }],
        warnings: vec![],
        acknowledged: vec![],
        monitor: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"passed\":false"));
    assert!(json.contains("\"acknowledged\":false"));
}

#[test]
fn check_result_with_warnings() {
    let result = CheckResult {
        passed: true,
        files_checked: 1,
        actor: "human".to_string(),
        enforced: false,
        tree_oid: None,
        check_set_version: None,
        check_set_age_seconds: None,
        blocking: vec![],
        warnings: vec![CheckMatch {
            id: "TEST-3".to_string(),
            file: "src/utils.rs".to_string(),
            target: "src/utils/".to_string(),
            message: "Consider reviewing utility changes".to_string(),
            severity: "warn".to_string(),
            acknowledged: false,
        }],
        acknowledged: vec![],
        monitor: vec![],
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
        acknowledged: true,
    };

    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("\"file\":\"test.rs\""));
    assert!(json.contains("\"severity\":\"warn\""));
    assert!(json.contains("\"acknowledged\":true"));
}

// =============================================================================
// CheckListResult Serialization Tests
// =============================================================================

#[test]
fn check_list_result_serialization() {
    let result = CheckListResult {
        checks: vec![CheckInfo {
            id: ".noslop.toml:0".to_string(),
            target: "src/**/*.rs".to_string(),
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
// AckResult Serialization Tests
// =============================================================================

#[test]
fn ack_result_serialization() {
    let result = AckResult {
        success: true,
        check_id: "session-check".to_string(),
        message: "Verified session handling".to_string(),
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("session-check"));
}

#[test]
fn ack_result_failure() {
    let result = AckResult {
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

// =============================================================================
// Tree oid + UploadEnvelope Serialization Tests
// =============================================================================

#[test]
fn check_result_tree_oid_serializes_when_present() {
    let result = CheckResult {
        passed: true,
        files_checked: 1,
        actor: "ci".to_string(),
        enforced: true,
        tree_oid: Some("8f2c4b1e".to_string()),
        check_set_version: None,
        check_set_age_seconds: None,
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![],
        monitor: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"tree_oid\":\"8f2c4b1e\""));
}

#[test]
fn check_result_tree_oid_omitted_when_absent() {
    let result = CheckResult {
        passed: true,
        files_checked: 0,
        actor: "human".to_string(),
        enforced: false,
        tree_oid: None,
        check_set_version: None,
        check_set_age_seconds: None,
        blocking: vec![],
        warnings: vec![],
        acknowledged: vec![],
        monitor: vec![],
    };

    // Additive schema-1 field: older-payload consumers never see the key
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("tree_oid"));
}

#[test]
fn upload_envelope_serializes_stable_field_names() {
    let envelope = UploadEnvelope {
        schema: ENVELOPE_SCHEMA,
        repo: "noslop-sh/noslop".to_string(),
        sha: "abc123".to_string(),
        pr: String::new(),
        base: "origin/main".to_string(),
        branch: "sp/no-2-matcher".to_string(),
        pr_title: String::new(),
        check: serde_json::json!({"passed": false, "blocking": [{"id": "NOS-2"}]}),
        ledger: vec![noslop::adapters::ledger::LedgerRecord {
            schema: 1,
            ack: noslop::core::models::Acknowledgment::new(
                "NOS-2".into(),
                "exact-ID matching only; added tests".into(),
                "claude-code".into(),
            ),
        }],
    };

    let json = serde_json::to_string(&envelope).unwrap();
    // The stable API surface: field names may not drift (docs/SCHEMA.md)
    assert!(json.contains("\"schema\":1"));
    assert!(json.contains("\"repo\":\"noslop-sh/noslop\""));
    assert!(json.contains("\"pr\":\"\""));
    assert!(json.contains("\"branch\":\"sp/no-2-matcher\""));
    // Additive empty fields stay off the wire entirely
    assert!(!json.contains("pr_title"));
    // check payload embedded verbatim
    assert!(json.contains("\"blocking\":[{\"id\":\"NOS-2\"}]"));
    // ledger record carries the justification and actor
    assert!(json.contains("\"check_id\":\"NOS-2\""));
    assert!(json.contains("\"acknowledged_by\":\"claude-code\""));
}
