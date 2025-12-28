//! Tests for storage module (trailer and file storage)

use noslop::models::Attestation;
use noslop::storage::{
    AttestationStore, Backend, TrailerAttestationStore, trailer::append_trailers,
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
    let store = TrailerAttestationStore::new();
    // Just verify it creates without panic
    let _ = store;
}

#[test]
fn test_trailer_store_default() {
    let store = TrailerAttestationStore;
    let _ = store;
}

#[test]
fn test_format_trailers_single() {
    let store = TrailerAttestationStore::new();
    let attestation =
        Attestation::new("AST-1".to_string(), "Reviewed the code".to_string(), "human".to_string());

    let trailers = store.format_trailers(&[attestation]);
    assert_eq!(trailers, "Noslop-Attest: AST-1 | Reviewed the code | human");
}

#[test]
fn test_format_trailers_multiple() {
    let store = TrailerAttestationStore::new();
    let attestations = vec![
        Attestation::new("AST-1".to_string(), "First review".to_string(), "human".to_string()),
        Attestation::new(
            "AST-2".to_string(),
            "Second review".to_string(),
            "claude-3-opus".to_string(),
        ),
    ];

    let trailers = store.format_trailers(&attestations);
    let lines: Vec<&str> = trailers.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Noslop-Attest: AST-1 | First review | human");
    assert_eq!(lines[1], "Noslop-Attest: AST-2 | Second review | claude-3-opus");
}

#[test]
fn test_format_trailers_empty() {
    let store = TrailerAttestationStore::new();
    let trailers = store.format_trailers(&[]);
    assert_eq!(trailers, "");
}

#[test]
fn test_format_trailers_escapes_pipe() {
    let store = TrailerAttestationStore::new();
    let attestation = Attestation::new(
        "AST-1".to_string(),
        "Message with | pipe character".to_string(),
        "human".to_string(),
    );

    let trailers = store.format_trailers(&[attestation]);
    // Pipe in message should be replaced with dash
    assert_eq!(trailers, "Noslop-Attest: AST-1 | Message with - pipe character | human");
}

// =============================================================================
// APPEND TRAILERS TESTS
// =============================================================================

#[test]
fn test_append_trailers_to_simple_message() {
    let message = "Add new feature";
    let trailers = "Noslop-Attest: AST-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    assert_eq!(result, "Add new feature\n\nNoslop-Attest: AST-1 | Reviewed | human");
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
    let trailers = "Noslop-Attest: AST-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    // Should append to existing trailers with just one newline
    assert!(result.contains("Signed-off-by:"));
    assert!(result.contains("Noslop-Attest:"));
}

#[test]
fn test_append_trailers_message_with_trailing_whitespace() {
    let message = "Add new feature   \n\n";
    let trailers = "Noslop-Attest: AST-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    // Should trim trailing whitespace
    assert!(result.starts_with("Add new feature"));
    assert!(result.contains("Noslop-Attest:"));
}

#[test]
fn test_append_trailers_multiline_message() {
    let message = "Add new feature\n\nThis is a longer description\nwith multiple lines.";
    let trailers = "Noslop-Attest: AST-1 | Reviewed | human";

    let result = append_trailers(message, trailers);
    assert!(result.contains("This is a longer description"));
    assert!(result.ends_with("Noslop-Attest: AST-1 | Reviewed | human"));
}

#[test]
fn test_append_multiple_trailers() {
    let message = "Add new feature";
    let trailers = "Noslop-Attest: AST-1 | First | human\nNoslop-Attest: AST-2 | Second | agent";

    let result = append_trailers(message, trailers);
    assert!(result.contains("AST-1"));
    assert!(result.contains("AST-2"));
}
