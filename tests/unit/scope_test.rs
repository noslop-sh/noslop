//! Tests for the Scope module
//!
//! Scope handles parsing and matching of file path specifications with optional
//! fragment identifiers (line numbers, ranges, symbols).

use noslop::scope::{Fragment, Scope};

// =============================================================================
// Parsing Tests
// =============================================================================

#[test]
fn parse_exact_path() {
    let s = Scope::parse("src/auth.rs").unwrap();
    assert!(!s.is_glob());
    assert_eq!(s.path_pattern(), "src/auth.rs");
    assert!(s.fragment().is_none());
}

#[test]
fn parse_glob_star() {
    let s = Scope::parse("*.rs").unwrap();
    assert!(s.is_glob());
    assert!(s.matches("foo.rs"));
    assert!(s.matches("bar.rs"));
    assert!(!s.matches("foo.py"));
    assert!(!s.matches("src/foo.rs")); // * doesn't match /
}

#[test]
fn parse_glob_doublestar() {
    let s = Scope::parse("src/**/*.rs").unwrap();
    assert!(s.is_glob());
    assert!(s.matches("src/foo.rs"));
    assert!(s.matches("src/auth/login.rs"));
    assert!(s.matches("src/auth/deep/nested/file.rs"));
    assert!(!s.matches("foo.rs")); // must be under src/
    assert!(!s.matches("src/foo.py"));
}

#[test]
fn parse_glob_single_dir() {
    let s = Scope::parse("src/*.rs").unwrap();
    assert!(s.matches("src/foo.rs"));
    assert!(!s.matches("src/auth/foo.rs")); // * doesn't cross /
}

#[test]
fn parse_glob_question_mark() {
    let s = Scope::parse("src/?.rs").unwrap();
    assert!(s.matches("src/a.rs"));
    assert!(s.matches("src/b.rs"));
    assert!(!s.matches("src/ab.rs")); // ? matches single char
}

#[test]
fn parse_trailing_doublestar() {
    let s = Scope::parse("src/**").unwrap();
    assert!(s.matches("src/foo.rs"));
    assert!(s.matches("src/auth/login.rs"));
}

// =============================================================================
// Fragment Parsing Tests
// =============================================================================

#[test]
fn parse_line_fragment() {
    let s = Scope::parse("src/auth.rs#L42").unwrap();
    assert_eq!(s.path_pattern(), "src/auth.rs");
    assert_eq!(s.fragment(), Some(&Fragment::Line(42)));
    assert!(s.matches("src/auth.rs"));
}

#[test]
fn parse_line_range_fragment() {
    let s = Scope::parse("src/auth.rs#L42-L50").unwrap();
    assert_eq!(s.fragment(), Some(&Fragment::LineRange(42, 50)));

    // Also support L42-50 format
    let s2 = Scope::parse("src/auth.rs#L10-20").unwrap();
    assert_eq!(s2.fragment(), Some(&Fragment::LineRange(10, 20)));
}

#[test]
fn parse_symbol_fragment() {
    let s = Scope::parse("src/auth.rs#Session").unwrap();
    assert_eq!(s.fragment(), Some(&Fragment::Symbol("Session".to_string())));

    let s2 = Scope::parse("src/auth.rs#Session.validate").unwrap();
    assert_eq!(s2.fragment(), Some(&Fragment::Symbol("Session.validate".to_string())));
}

#[test]
fn invalid_line_range() {
    let result = Scope::parse("src/auth.rs#L50-L10");
    assert!(result.is_err());
}

#[test]
fn empty_scope() {
    let result = Scope::parse("");
    assert!(result.is_err());
}

// =============================================================================
// Path Matching Tests
// =============================================================================

#[test]
fn exact_path_matching() {
    let s = Scope::parse("src/auth").unwrap();
    assert!(s.matches("src/auth"));
    assert!(s.matches("src/auth/login.rs")); // prefix match
    assert!(!s.matches("src/authorization"));
}

#[test]
fn directory_pattern() {
    let s = Scope::parse("src/auth/").unwrap();
    assert!(s.matches("src/auth/login.rs"));
    assert!(s.matches("src/auth/session.rs"));
}

#[test]
fn whitespace_handling() {
    let s = Scope::parse("  src/auth.rs  ").unwrap();
    assert_eq!(s.path_pattern(), "src/auth.rs");
}

// =============================================================================
// Display and Accessor Tests
// =============================================================================

#[test]
fn display_scope() {
    let s = Scope::parse("src/**/*.rs#L42").unwrap();
    assert_eq!(s.to_string(), "src/**/*.rs#L42");
}

#[test]
fn display_fragment() {
    assert_eq!(Fragment::Line(42).to_string(), "L42");
    assert_eq!(Fragment::LineRange(10, 20).to_string(), "L10-L20");
    assert_eq!(Fragment::Symbol("Foo".to_string()).to_string(), "Foo");
}

#[test]
fn fragment_accessors() {
    let s1 = Scope::parse("file.rs#L42").unwrap();
    assert_eq!(s1.fragment().unwrap().line(), Some(42));
    assert_eq!(s1.fragment().unwrap().line_range(), None);
    assert_eq!(s1.fragment().unwrap().symbol(), None);

    let s2 = Scope::parse("file.rs#L10-L20").unwrap();
    assert_eq!(s2.fragment().unwrap().line(), None);
    assert_eq!(s2.fragment().unwrap().line_range(), Some((10, 20)));

    let s3 = Scope::parse("file.rs#MyClass").unwrap();
    assert_eq!(s3.fragment().unwrap().symbol(), Some("MyClass"));
}

#[test]
fn has_fragment() {
    let s1 = Scope::parse("file.rs").unwrap();
    assert!(!s1.has_fragment());

    let s2 = Scope::parse("file.rs#L42").unwrap();
    assert!(s2.has_fragment());
}
