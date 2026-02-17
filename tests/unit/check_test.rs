//! Tests for Check model

use noslop::core::models::{Check, Severity, Target};

fn make_check(target_str: &str) -> Check {
    Check::new("TEST-1", Target::pattern(target_str), "Test message", Severity::Block)
}

mod applies_to {
    use super::*;

    #[test]
    fn exact_path_matches() {
        let check = make_check("src/auth.rs");
        assert!(check.applies_to("src/auth.rs"));
    }

    #[test]
    fn exact_path_rejects_different() {
        let check = make_check("src/auth.rs");
        assert!(!check.applies_to("src/main.rs"));
    }

    #[test]
    fn glob_star_matches_extension() {
        let check = make_check("*.rs");
        assert!(check.applies_to("main.rs"));
        assert!(check.applies_to("lib.rs"));
    }

    #[test]
    fn glob_star_rejects_different_extension() {
        let check = make_check("*.rs");
        assert!(!check.applies_to("main.py"));
    }

    #[test]
    fn glob_doublestar_matches_nested() {
        let check = make_check("src/**/*.rs");
        assert!(check.applies_to("src/auth.rs"));
        assert!(check.applies_to("src/auth/login.rs"));
        assert!(check.applies_to("src/auth/handlers/oauth.rs"));
    }

    #[test]
    fn glob_doublestar_rejects_outside() {
        let check = make_check("src/**/*.rs");
        assert!(!check.applies_to("tests/auth.rs"));
    }

    #[test]
    fn glob_single_dir_star() {
        let check = make_check("src/*.rs");
        assert!(check.applies_to("src/auth.rs"));
        assert!(!check.applies_to("src/auth/login.rs")); // nested should not match
    }

    #[test]
    fn glob_question_mark() {
        let check = make_check("src/?.rs");
        assert!(check.applies_to("src/a.rs"));
        assert!(!check.applies_to("src/ab.rs"));
    }
}
