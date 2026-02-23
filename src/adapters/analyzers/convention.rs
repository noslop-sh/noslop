//! Convention analyzer
//!
//! Matches `[[check]]` rules from `.noslop.toml` against changed files in a
//! review diff. Each matching check produces a [`Feedback`] via
//! [`Check::into_feedback`].

use crate::core::models::{Check, Feedback};
use crate::core::ports::{AnalyzerTier, ContextKind, ReviewAnalyzer, ReviewContext};

/// Produces feedbacks from `[[check]]` entries in `.noslop.toml`.
///
/// For each check, the analyzer tests every changed file against
/// [`Check::applies_to`]. Matching pairs are converted to feedbacks
/// via [`Check::into_feedback`].
pub struct ConventionAnalyzer {
    checks: Vec<Check>,
}

impl ConventionAnalyzer {
    /// Create a new convention analyzer with the given checks.
    #[must_use]
    pub const fn new(checks: Vec<Check>) -> Self {
        Self { checks }
    }
}

#[allow(clippy::unnecessary_literal_bound)]
impl ReviewAnalyzer for ConventionAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::ChangedFiles]
    }

    fn analyze(
        &self,
        context: &ReviewContext,
        _prior_feedbacks: &[Feedback],
    ) -> anyhow::Result<Vec<Feedback>> {
        let mut feedbacks = Vec::new();
        for check in &self.checks {
            for file in &context.changed_files {
                if check.applies_to(file) {
                    feedbacks.push(check.clone().into_feedback(file));
                }
            }
        }
        Ok(feedbacks)
    }

    fn name(&self) -> &str {
        "conventions"
    }

    fn tier(&self) -> AnalyzerTier {
        AnalyzerTier::Static
    }
}

impl std::fmt::Debug for ConventionAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConventionAnalyzer")
            .field("checks_count", &self.checks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{FeedbackSource, Severity, Target};
    use std::path::PathBuf;

    fn test_context(changed_files: Vec<&str>) -> ReviewContext {
        ReviewContext {
            diff: String::new(),
            changed_files: changed_files.into_iter().map(String::from).collect(),
            repo_root: PathBuf::from("/tmp/test"),
            commit_range: None,
            base: "main".to_string(),
            head: "HEAD".to_string(),
        }
    }

    #[test]
    fn no_checks_produces_no_feedbacks() {
        let analyzer = ConventionAnalyzer::new(vec![]);
        let ctx = test_context(vec!["src/main.rs"]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(feedbacks.is_empty());
    }

    #[test]
    fn no_changed_files_produces_no_feedbacks() {
        let checks = vec![Check::new(
            "NOS-1",
            Target::pattern("src/**/*.rs"),
            "Review Rust files",
            Severity::Block,
        )];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec![]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(feedbacks.is_empty());
    }

    #[test]
    fn exact_match_produces_feedback() {
        let checks = vec![Check::new(
            "NOS-1",
            Target::file("src/auth.rs"),
            "Session handling changed",
            Severity::Block,
        )];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec!["src/auth.rs", "src/main.rs"]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert_eq!(feedbacks.len(), 1);
        assert_eq!(feedbacks[0].target.path, "src/auth.rs");
        assert_eq!(feedbacks[0].severity, Severity::Block);
        assert_eq!(feedbacks[0].message, "Session handling changed");
    }

    #[test]
    fn exact_match_no_hit() {
        let checks = vec![Check::new(
            "NOS-1",
            Target::file("src/auth.rs"),
            "Session handling changed",
            Severity::Block,
        )];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec!["src/main.rs", "src/lib.rs"]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(feedbacks.is_empty());
    }

    #[test]
    fn glob_match_produces_feedbacks() {
        let checks = vec![Check::new(
            "NOS-2",
            Target::pattern("src/adapters/**/*.rs"),
            "Adapter changed -- verify port trait contract",
            Severity::Warn,
        )];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec![
            "src/adapters/git/hooks.rs",
            "src/adapters/toml/parser.rs",
            "src/core/models/check.rs",
        ]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert_eq!(feedbacks.len(), 2);
        assert!(feedbacks.iter().all(|f| f.severity == Severity::Warn));
        let paths: Vec<&str> = feedbacks.iter().map(|f| f.target.path.as_str()).collect();
        assert!(paths.contains(&"src/adapters/git/hooks.rs"));
        assert!(paths.contains(&"src/adapters/toml/parser.rs"));
    }

    #[test]
    fn multiple_checks_multiple_files() {
        let checks = vec![
            Check::new("NOS-1", Target::file("src/auth.rs"), "Auth changed", Severity::Block),
            Check::new(
                "NOS-2",
                Target::pattern("src/**/*.rs"),
                "Rust file changed",
                Severity::Info,
            ),
        ];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec!["src/auth.rs", "src/main.rs"]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        // NOS-1 matches src/auth.rs (1 feedback)
        // NOS-2 matches src/auth.rs and src/main.rs (2 feedbacks)
        assert_eq!(feedbacks.len(), 3);
    }

    #[test]
    fn feedback_source_is_check() {
        let checks =
            vec![Check::new("NOS-5", Target::file("src/auth.rs"), "Review auth", Severity::Block)];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec!["src/auth.rs"]);
        let feedbacks = analyzer.analyze(&ctx, &[]).unwrap();
        assert_eq!(feedbacks.len(), 1);
        assert_eq!(feedbacks[0].source, FeedbackSource::Check("NOS-5".into()));
    }

    #[test]
    fn prior_feedbacks_ignored() {
        // ConventionAnalyzer is a static tier and does not use prior_feedbacks
        let checks =
            vec![Check::new("NOS-1", Target::file("src/auth.rs"), "Auth", Severity::Block)];
        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = test_context(vec!["src/auth.rs"]);

        let prior = vec![Feedback::new(
            Target::file("src/other.rs"),
            Severity::Warn,
            "Prior feedback",
            FeedbackSource::Script("lint".into()),
        )];

        let feedbacks = analyzer.analyze(&ctx, &prior).unwrap();
        assert_eq!(feedbacks.len(), 1);
        // The feedback is for auth.rs, not influenced by prior
        assert_eq!(feedbacks[0].target.path, "src/auth.rs");
    }

    #[test]
    fn analyzer_metadata() {
        let analyzer = ConventionAnalyzer::new(vec![]);
        assert_eq!(analyzer.name(), "conventions");
        assert_eq!(analyzer.tier(), AnalyzerTier::Static);
        assert_eq!(analyzer.required_context(), vec![ContextKind::ChangedFiles]);
    }

    #[test]
    fn debug_impl() {
        let analyzer = ConventionAnalyzer::new(vec![Check::new(
            "NOS-1",
            Target::file("src/auth.rs"),
            "Test",
            Severity::Block,
        )]);
        let debug = format!("{analyzer:?}");
        assert!(debug.contains("ConventionAnalyzer"));
        assert!(debug.contains("checks_count: 1"));
    }
}
