//! Check model
//!
//! A check declares: "When files matching `target` change, surface this message."
//! Checks are static rules from `.noslop.toml`. The convention analyzer matches
//! checks against changed files and calls `into_finding()` to bridge the two worlds.

use serde::{Deserialize, Serialize};

use super::finding::Finding;
use super::primitives::{FindingSource, Severity, Target};

/// A review rule from `.noslop.toml`.
///
/// Declares: "when files matching `target` change, surface this message."
///
/// ```toml
/// [[check]]
/// id = "NOS-1"
/// target = "src/adapters/git/hooks.rs"
/// message = "Verify hook installation still works"
/// severity = "block"
/// tags = ["hooks"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Unique identifier (e.g., "NOS-1"). Set in TOML or auto-generated.
    pub id: String,
    /// File path or glob pattern to match against changed files.
    pub target: Target,
    /// What the reviewer should verify when matched files change.
    pub message: String,
    /// How strictly this check is enforced.
    pub severity: Severity,
    /// Optional tags for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Check {
    /// Create a new check.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        target: Target,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: id.into(),
            target,
            message: message.into(),
            severity,
            tags: Vec::new(),
        }
    }

    /// Add tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Whether this check's target matches a given file path.
    #[must_use]
    pub fn applies_to(&self, path: &str) -> bool {
        self.target.matches(path)
    }

    /// Convert this check into a Finding for a specific matched file.
    ///
    /// This is the bridge between the check world and the finding world.
    /// The convention analyzer calls this for each (check, `changed_file`) pair.
    #[must_use]
    pub fn into_finding(self, matched_path: &str) -> Finding {
        let source = FindingSource::Check(self.id.clone());
        Finding::new(Target::file(matched_path), self.severity, self.message, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_new() {
        let check =
            Check::new("NOS-1", Target::pattern("src/**/*.rs"), "Review this", Severity::Block);
        assert_eq!(check.id, "NOS-1");
        assert!(check.target.is_glob());
        assert_eq!(check.message, "Review this");
        assert_eq!(check.severity, Severity::Block);
        assert!(check.tags.is_empty());
    }

    #[test]
    fn check_with_tags() {
        let check = Check::new("NOS-1", Target::pattern("*.rs"), "msg", Severity::Block)
            .with_tags(vec!["hooks".into(), "security".into()]);
        assert_eq!(check.tags.len(), 2);
    }

    #[test]
    fn check_applies_to_exact() {
        let check = Check::new("NOS-1", Target::file("src/auth.rs"), "msg", Severity::Block);
        assert!(check.applies_to("src/auth.rs"));
        assert!(!check.applies_to("src/main.rs"));
    }

    #[test]
    fn check_applies_to_glob() {
        let check = Check::new("NOS-1", Target::pattern("src/**/*.rs"), "msg", Severity::Block);
        assert!(check.applies_to("src/core/models/review.rs"));
        assert!(!check.applies_to("tests/unit/check_test.rs"));
    }

    #[test]
    fn check_into_finding() {
        let check = Check::new(
            "NOS-5",
            Target::pattern("src/adapters/**/*.rs"),
            "Adapter changed -- verify port trait contract",
            Severity::Block,
        );
        let finding = check.into_finding("src/adapters/git/hooks.rs");
        assert_eq!(finding.target.path, "src/adapters/git/hooks.rs");
        assert_eq!(finding.severity, Severity::Block);
        assert!(finding.is_blocking());
    }

    #[test]
    fn check_serde_roundtrip() {
        let check = Check::new("NOS-1", Target::pattern("*.rs"), "Test", Severity::Block)
            .with_tags(vec!["test".into()]);
        let json = serde_json::to_string(&check).unwrap();
        let parsed: Check = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "NOS-1");
        assert_eq!(parsed.tags, vec!["test"]);
    }
}
