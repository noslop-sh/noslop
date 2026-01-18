//! Tests for storage module (trailer and file storage)

use noslop::models::Verification;
use noslop::storage::{
    Backend, TrailerVerificationStore, VerificationStore, trailer::append_trailers,
};

// =============================================================================
// BACKEND TESTS
// =============================================================================

#[test]
fn test_backend_from_str_trailer() {
    assert_eq!("trailer".parse::<Backend>().unwrap(), Backend::Trailer);
    assert_eq!("trailers".parse::<Backend>().unwrap(), Backend::Trailer);
    assert_eq!("TRAILER".parse::<Backend>().unwrap(), Backend::Trailer);
}

#[test]
fn test_backend_from_str_file() {
    assert_eq!("file".parse::<Backend>().unwrap(), Backend::File);
    assert_eq!("files".parse::<Backend>().unwrap(), Backend::File);
    assert_eq!("FILE".parse::<Backend>().unwrap(), Backend::File);
}

#[test]
fn test_backend_from_str_unknown() {
    let result = "unknown".parse::<Backend>();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown backend"));
}

#[test]
fn test_backend_default() {
    assert_eq!(Backend::default(), Backend::Trailer);
}

// =============================================================================
// TRAILER STORE TESTS
// =============================================================================

#[test]
fn test_trailer_store_new() {
    let store = TrailerVerificationStore::new();
    // Just verify it creates without panic
    let _ = store;
}

#[test]
fn test_trailer_store_default() {
    let store = TrailerVerificationStore;
    let _ = store;
}

#[test]
fn test_format_trailers_single() {
    let store = TrailerVerificationStore::new();
    let verification = Verification::new(
        "NOS-1".to_string(),
        "Reviewed the code".to_string(),
        "human".to_string(),
    );

    let trailers = store.format_trailers(&[verification]);
    assert_eq!(trailers, "Noslop-Verify: NOS-1 | Reviewed the code | human");
}

#[test]
fn test_format_trailers_multiple() {
    let store = TrailerVerificationStore::new();
    let verifications = vec![
        Verification::new("NOS-1".to_string(), "First review".to_string(), "human".to_string()),
        Verification::new(
            "NOS-2".to_string(),
            "Second review".to_string(),
            "claude-3-opus".to_string(),
        ),
    ];

    let trailers = store.format_trailers(&verifications);
    let lines: Vec<&str> = trailers.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Noslop-Verify: NOS-1 | First review | human");
    assert_eq!(lines[1], "Noslop-Verify: NOS-2 | Second review | claude-3-opus");
}

#[test]
fn test_format_trailers_empty() {
    let store = TrailerVerificationStore::new();
    let trailers = store.format_trailers(&[]);
    assert_eq!(trailers, "");
}

#[test]
fn test_format_trailers_escapes_pipe() {
    let store = TrailerVerificationStore::new();
    let verification = Verification::new(
        "NOS-1".to_string(),
        "Message with | pipe character".to_string(),
        "human".to_string(),
    );

    let trailers = store.format_trailers(&[verification]);
    // Pipe in message should be replaced with dash
    assert_eq!(trailers, "Noslop-Verify: NOS-1 | Message with - pipe character | human");
}

// =============================================================================
// APPEND TRAILERS TESTS
// =============================================================================

#[test]
fn test_append_trailers_to_simple_message() {
    let message = "Add new feature";
    let trailers = "Noslop-Verify: NOS-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    assert_eq!(result, "Add new feature\n\nNoslop-Verify: NOS-1 | Reviewed | human");
}

#[test]
fn test_append_trailers_empty_trailers() {
    let message = "Add new feature";
    let trailers = "";

    let result = append_trailers(message, trailers);
    assert_eq!(result, "Add new feature");
}

#[test]
fn test_append_trailers_message_with_existing_trailers() {
    let message = "Add new feature\n\nSigned-off-by: Alice <alice@example.com>";
    let trailers = "Noslop-Verify: NOS-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    // Should append to existing trailers with just one newline
    assert!(result.contains("Signed-off-by:"));
    assert!(result.contains("Noslop-Verify:"));
}

#[test]
fn test_append_trailers_message_with_trailing_whitespace() {
    let message = "Add new feature   \n\n";
    let trailers = "Noslop-Verify: NOS-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    // Should trim trailing whitespace
    assert!(result.starts_with("Add new feature"));
    assert!(result.contains("Noslop-Verify:"));
}

#[test]
fn test_append_trailers_multiline_message() {
    let message = "Add new feature\n\nThis is a longer description\nwith multiple lines.";
    let trailers = "Noslop-Verify: NOS-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    assert!(result.contains("This is a longer description"));
    assert!(result.ends_with("Noslop-Verify: NOS-1 | Reviewed | human"));
}

#[test]
fn test_append_multiple_trailers() {
    let message = "Add new feature";
    let trailers = "Noslop-Verify: NOS-1 | First | human\nNoslop-Verify: NOS-2 | Second | agent";

    let result = append_trailers(message, trailers);
    assert!(result.contains("NOS-1"));
    assert!(result.contains("NOS-2"));
}

// =============================================================================
// TRAILER PARSING TESTS
// =============================================================================

mod trailer_parsing_tests {
    use noslop::storage::trailer::parse_verification_trailer;

    #[test]
    fn test_parse_verification_trailer_full() {
        let value = "NOS-1 | Fixed the bug | human";
        let result = parse_verification_trailer(value);

        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.check_id, "NOS-1");
        assert_eq!(v.message, "Fixed the bug");
        assert_eq!(v.verified_by, "human");
    }

    #[test]
    fn test_parse_verification_trailer_with_llm_verifier() {
        let value = "CHK-42 | Analyzed security implications | claude-3-opus";
        let result = parse_verification_trailer(value);

        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.check_id, "CHK-42");
        assert_eq!(v.message, "Analyzed security implications");
        assert_eq!(v.verified_by, "claude-3-opus");
    }

    #[test]
    fn test_parse_verification_trailer_missing_verifier() {
        // When verifier is missing, it defaults to "unknown"
        let value = "NOS-1 | Just the message";
        let result = parse_verification_trailer(value);

        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.check_id, "NOS-1");
        assert_eq!(v.message, "Just the message");
        assert_eq!(v.verified_by, "unknown");
    }

    #[test]
    fn test_parse_verification_trailer_invalid_format() {
        // No pipe separator at all
        let value = "NOS-1 Fixed the bug";
        let result = parse_verification_trailer(value);

        // Should return None for invalid format
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_verification_trailer_with_extra_whitespace() {
        let value = "  NOS-1  |   Message with spaces   |  human  ";
        let result = parse_verification_trailer(value);

        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.check_id, "NOS-1");
        assert_eq!(v.message, "Message with spaces");
        assert_eq!(v.verified_by, "human");
    }
}
