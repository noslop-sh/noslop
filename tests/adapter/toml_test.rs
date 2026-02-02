//! Tests for TOML adapter (assertion repository)

use noslop::adapters::toml::{AssertionEntry, find_noslop_files, load_file};
use std::fs;
use tempfile::TempDir;

// =============================================================================
// NOSLOP FILE PARSING TESTS
// =============================================================================

#[test]
fn test_parse_empty_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".noslop.toml");
    fs::write(&path, "# Empty noslop file\n").unwrap();

    let file = load_file(&path).unwrap();
    assert!(file.assertions.is_empty());
}

#[test]
fn test_parse_single_assertion() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".noslop.toml");
    fs::write(
        &path,
        r#"
[[assert]]
target = "*.rs"
message = "Rust code must be reviewed"
severity = "block"
"#,
    )
    .unwrap();

    let file = load_file(&path).unwrap();
    assert_eq!(file.assertions.len(), 1);
    assert_eq!(file.assertions[0].target, "*.rs");
    assert_eq!(file.assertions[0].message, "Rust code must be reviewed");
    assert_eq!(file.assertions[0].severity, "block");
}

#[test]
fn test_parse_multiple_assertions() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".noslop.toml");
    fs::write(
        &path,
        r#"
[[assert]]
target = "*.rs"
message = "Review Rust code"
severity = "block"

[[assert]]
target = "*.py"
message = "Review Python code"
severity = "warn"
"#,
    )
    .unwrap();

    let file = load_file(&path).unwrap();
    assert_eq!(file.assertions.len(), 2);
    assert_eq!(file.assertions[0].severity, "block");
    assert_eq!(file.assertions[1].severity, "warn");
}

#[test]
fn test_parse_assertion_with_tags() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".noslop.toml");
    fs::write(
        &path,
        r#"
[[assert]]
target = "*.rs"
message = "Security review required"
severity = "block"
tags = ["security", "audit"]
"#,
    )
    .unwrap();

    let file = load_file(&path).unwrap();
    assert_eq!(file.assertions[0].tags.len(), 2);
    assert!(file.assertions[0].tags.contains(&"security".to_string()));
}

// =============================================================================
// FILE DISCOVERY TESTS
// =============================================================================

#[test]
fn test_find_noslop_files_in_root() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".noslop.toml");
    fs::write(&path, "# noslop").unwrap();

    let files = find_noslop_files(temp.path());
    assert_eq!(files.len(), 1);
}

#[test]
fn test_find_noslop_files_walks_up_hierarchy() {
    let temp = TempDir::new().unwrap();

    // Root file
    fs::write(temp.path().join(".noslop.toml"), "# root").unwrap();

    // Nested file
    let sub = temp.path().join("src");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".noslop.toml"), "# src").unwrap();

    // When searching FROM nested dir, should find both files walking UP
    let files = find_noslop_files(&sub);
    assert_eq!(files.len(), 2);

    // When searching FROM root, should find only root file
    let files = find_noslop_files(temp.path());
    assert_eq!(files.len(), 1);
}

#[test]
fn test_find_noslop_files_none_found() {
    let temp = TempDir::new().unwrap();
    let files = find_noslop_files(temp.path());
    assert!(files.is_empty());
}

// =============================================================================
// ASSERTION ENTRY TESTS
// =============================================================================

#[test]
fn test_assertion_entry_fields() {
    let entry = AssertionEntry {
        id: Some("TEST-1".to_string()),
        target: "*.rs".to_string(),
        message: "Review".to_string(),
        severity: "block".to_string(),
        tags: vec!["security".to_string()],
    };

    assert_eq!(entry.id, Some("TEST-1".to_string()));
    assert_eq!(entry.target, "*.rs");
    assert!(entry.tags.contains(&"security".to_string()));
}

#[test]
fn test_assertion_entry_optional_id() {
    let entry = AssertionEntry {
        id: None,
        target: "*.py".to_string(),
        message: "Review Python".to_string(),
        severity: "warn".to_string(),
        tags: vec![],
    };

    assert!(entry.id.is_none());
    assert!(entry.tags.is_empty());
}
