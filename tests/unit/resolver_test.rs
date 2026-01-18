//! Tests for the Resolver module
//!
//! Resolver finds files matching Scope specifications and extracts content
//! based on fragment identifiers.

use std::path::PathBuf;

use noslop::resolver::{ResolveError, Resolver};
use noslop::scope::Scope;

use crate::common::TestRepo;

// =============================================================================
// Resolver Construction Tests
// =============================================================================

#[test]
fn resolver_new() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    assert_eq!(resolver.root(), repo.path());
}

#[test]
fn resolver_invalid_path() {
    let result = Resolver::new("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
}

// =============================================================================
// File Finding Tests
// =============================================================================

#[test]
fn find_exact_file() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("src/auth/login.rs").unwrap();
    let files = resolver.find_files(&scope).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], PathBuf::from("src/auth/login.rs"));
}

#[test]
fn find_glob_pattern() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("src/**/*.rs").unwrap();
    let files = resolver.find_files(&scope).unwrap();
    assert_eq!(files.len(), 3);
    assert!(files.contains(&PathBuf::from("src/auth/login.rs")));
    assert!(files.contains(&PathBuf::from("src/auth/session.rs")));
    assert!(files.contains(&PathBuf::from("src/api/routes.rs")));
}

#[test]
fn find_extension_glob() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("*.md").unwrap();
    let files = resolver.find_files(&scope).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], PathBuf::from("README.md"));
}

#[test]
fn hidden_files_excluded() {
    let repo = TestRepo::new();
    repo.add_hidden_file(".hidden", "secret");
    repo.add_hidden_dir(".hidden_dir", "file.rs", "hidden");

    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("*").unwrap();
    let files = resolver.find_files(&scope).unwrap();

    // Should not include hidden files
    assert!(!files.contains(&PathBuf::from(".hidden")));
}

// =============================================================================
// Content Resolution Tests
// =============================================================================

#[test]
fn resolve_with_line_fragment() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("src/auth/login.rs#L2").unwrap();
    let files = resolver.resolve(&scope).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].content_str(), Some("    // line 2"));
}

#[test]
fn resolve_with_line_range() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("src/auth/login.rs#L2-L3").unwrap();
    let files = resolver.resolve(&scope).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].content_str(), Some("    // line 2\n    // line 3"));
}

#[test]
fn resolve_line_out_of_range() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("src/auth/login.rs#L100").unwrap();
    let result = resolver.resolve(&scope);
    assert!(matches!(result, Err(ResolveError::LineOutOfRange(100, _))));
}

#[test]
fn resolve_without_fragment_no_content() {
    // Resolver only extracts content when there's a fragment
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("README.md").unwrap();
    let files = resolver.resolve(&scope).unwrap();
    assert_eq!(files.len(), 1);
    assert!(!files[0].has_content()); // No fragment means no content extraction
}

#[test]
fn read_file_returns_content() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let content = resolver.read_file("README.md").unwrap();
    assert_eq!(content, "# Test\n");
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn resolve_nonexistent_file() {
    let repo = TestRepo::new();
    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("nonexistent.rs").unwrap();
    let files = resolver.find_files(&scope).unwrap();
    assert!(files.is_empty());
}

#[test]
fn resolve_empty_directory() {
    let repo = TestRepo::new();
    std::fs::create_dir_all(repo.path().join("empty")).unwrap();

    let resolver = Resolver::new(repo.path()).unwrap();
    let scope = Scope::parse("empty/*").unwrap();
    let files = resolver.find_files(&scope).unwrap();
    assert!(files.is_empty());
}
