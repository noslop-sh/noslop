//! Tests for Verification model

use noslop::models::Verification;

#[test]
fn test_verification_new() {
    let v =
        Verification::new("NOS-1".to_string(), "Fixed the issue".to_string(), "human".to_string());

    assert_eq!(v.check_id, "NOS-1");
    assert_eq!(v.message, "Fixed the issue");
    assert_eq!(v.verified_by, "human");
    assert!(!v.created_at.is_empty());
}

#[test]
fn test_verification_by_human() {
    let v = Verification::by_human("NOS-2".to_string(), "Reviewed carefully".to_string());

    assert_eq!(v.check_id, "NOS-2");
    assert_eq!(v.message, "Reviewed carefully");
    assert_eq!(v.verified_by, "human");
}

#[test]
fn test_verification_by_llm() {
    let v =
        Verification::by_llm("NOS-3".to_string(), "Analysis complete".to_string(), "claude-3-opus");

    assert_eq!(v.check_id, "NOS-3");
    assert_eq!(v.message, "Analysis complete");
    assert_eq!(v.verified_by, "claude-3-opus");
}

#[test]
fn test_verification_serialize() {
    let v = Verification::new("NOS-1".to_string(), "Test message".to_string(), "human".to_string());

    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("\"check_id\":\"NOS-1\""));
    assert!(json.contains("\"message\":\"Test message\""));
    assert!(json.contains("\"verified_by\":\"human\""));
    assert!(json.contains("\"created_at\":"));
}

#[test]
fn test_verification_deserialize() {
    let json = r#"{
        "check_id": "NOS-5",
        "message": "Deserialized test",
        "verified_by": "gpt-4",
        "created_at": "2024-01-01T00:00:00Z"
    }"#;

    let v: Verification = serde_json::from_str(json).unwrap();
    assert_eq!(v.check_id, "NOS-5");
    assert_eq!(v.message, "Deserialized test");
    assert_eq!(v.verified_by, "gpt-4");
    assert_eq!(v.created_at, "2024-01-01T00:00:00Z");
}
