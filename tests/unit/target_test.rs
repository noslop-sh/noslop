//! Tests for the Target module
//!
//! Target handles parsing and matching of file path specifications with optional
//! fragment identifiers (line numbers, ranges, symbols).

use noslop::core::models::{Fragment, Target};

// =============================================================================
// Parsing Tests
// =============================================================================

#[test]
fn parse_exact_path() {
    let t = Target::parse("src/auth.rs").unwrap();
    assert!(!t.is_glob());
    assert_eq!(t.path_pattern(), "src/auth.rs");
    assert!(t.fragment().is_none());
}

#[test]
fn parse_glob_star() {
    let t = Target::parse("*.rs").unwrap();
    assert!(t.is_glob());
    assert!(t.matches("foo.rs"));
    assert!(t.matches("bar.rs"));
    assert!(!t.matches("foo.py"));
    assert!(!t.matches("src/foo.rs")); // * doesn't match /
}

#[test]
fn parse_glob_doublestar() {
    let t = Target::parse("src/**/*.rs").unwrap();
    assert!(t.is_glob());
    assert!(t.matches("src/foo.rs"));
    assert!(t.matches("src/auth/login.rs"));
    assert!(t.matches("src/auth/deep/nested/file.rs"));
    assert!(!t.matches("foo.rs")); // must be under src/
    assert!(!t.matches("src/foo.py"));
}

#[test]
fn parse_glob_single_dir() {
    let t = Target::parse("src/*.rs").unwrap();
    assert!(t.matches("src/foo.rs"));
    assert!(!t.matches("src/auth/foo.rs")); // * doesn't cross /
}

#[test]
fn parse_glob_question_mark() {
    let t = Target::parse("src/?.rs").unwrap();
    assert!(t.matches("src/a.rs"));
    assert!(t.matches("src/b.rs"));
    assert!(!t.matches("src/ab.rs")); // ? matches single char
}

#[test]
fn parse_trailing_doublestar() {
    let t = Target::parse("src/**").unwrap();
    assert!(t.matches("src/foo.rs"));
    assert!(t.matches("src/auth/login.rs"));
}

// =============================================================================
// Fragment Parsing Tests
// =============================================================================

#[test]
fn parse_line_fragment() {
    let t = Target::parse("src/auth.rs#L42").unwrap();
    assert_eq!(t.path_pattern(), "src/auth.rs");
    assert_eq!(t.fragment(), Some(&Fragment::Line(42)));
    assert!(t.matches("src/auth.rs"));
}

#[test]
fn parse_line_range_fragment() {
    let t = Target::parse("src/auth.rs#L42-L50").unwrap();
    assert_eq!(t.fragment(), Some(&Fragment::LineRange(42, 50)));

    // Also support L42-50 format
    let t2 = Target::parse("src/auth.rs#L10-20").unwrap();
    assert_eq!(t2.fragment(), Some(&Fragment::LineRange(10, 20)));
}

#[test]
fn parse_symbol_fragment() {
    let t = Target::parse("src/auth.rs#Session").unwrap();
    assert_eq!(t.fragment(), Some(&Fragment::Symbol("Session".to_string())));

    let t2 = Target::parse("src/auth.rs#Session.validate").unwrap();
    assert_eq!(t2.fragment(), Some(&Fragment::Symbol("Session.validate".to_string())));
}

#[test]
fn invalid_line_range() {
    let result = Target::parse("src/auth.rs#L50-L10");
    assert!(result.is_err());
}

#[test]
fn empty_target() {
    let result = Target::parse("");
    assert!(result.is_err());
}

// =============================================================================
// Path Matching Tests
// =============================================================================

#[test]
fn exact_path_matching() {
    let t = Target::parse("src/auth").unwrap();
    assert!(t.matches("src/auth"));
    assert!(t.matches("src/auth/login.rs")); // prefix match
    assert!(!t.matches("src/authorization"));
}

#[test]
fn directory_pattern() {
    let t = Target::parse("src/auth/").unwrap();
    assert!(t.matches("src/auth/login.rs"));
    assert!(t.matches("src/auth/session.rs"));
}

#[test]
fn whitespace_handling() {
    let t = Target::parse("  src/auth.rs  ").unwrap();
    assert_eq!(t.path_pattern(), "src/auth.rs");
}

// =============================================================================
// Display and Accessor Tests
// =============================================================================

#[test]
fn display_target() {
    let t = Target::parse("src/**/*.rs#L42").unwrap();
    assert_eq!(t.to_string(), "src/**/*.rs#L42");
}

#[test]
fn display_fragment() {
    assert_eq!(Fragment::Line(42).to_string(), "L42");
    assert_eq!(Fragment::LineRange(10, 20).to_string(), "L10-L20");
    assert_eq!(Fragment::Symbol("Foo".to_string()).to_string(), "Foo");
}

#[test]
fn fragment_accessors() {
    let t1 = Target::parse("file.rs#L42").unwrap();
    assert_eq!(t1.fragment().unwrap().line(), Some(42));
    assert_eq!(t1.fragment().unwrap().line_range(), None);
    assert_eq!(t1.fragment().unwrap().symbol(), None);

    let t2 = Target::parse("file.rs#L10-L20").unwrap();
    assert_eq!(t2.fragment().unwrap().line(), None);
    assert_eq!(t2.fragment().unwrap().line_range(), Some((10, 20)));

    let t3 = Target::parse("file.rs#MyClass").unwrap();
    assert_eq!(t3.fragment().unwrap().symbol(), Some("MyClass"));
}

#[test]
fn has_fragment() {
    let t1 = Target::parse("file.rs").unwrap();
    assert!(!t1.has_fragment());

    let t2 = Target::parse("file.rs#L42").unwrap();
    assert!(t2.has_fragment());
}
