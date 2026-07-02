//! Proposal model
//!
//! A proposal is a candidate check discovered from rules files or review
//! history. Proposals never enforce anything: a human must accept one
//! (promoting it to a check in `.noslop.toml`) before it gates commits.

use serde::{Deserialize, Serialize};

use super::severity::Severity;

/// A candidate check awaiting human review
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// Glob pattern the check would apply to
    pub target: String,

    /// The obligation, phrased so an agent can act on it at commit time
    pub message: String,

    /// Proposed severity (human can change at review time)
    pub severity: Severity,

    /// Where this proposal came from, e.g. `CLAUDE.md:12` or `mining:fastapi`
    pub source: String,
}

impl Proposal {
    /// Normalized key used to dedupe proposals against each other and
    /// against existing checks: lowercased message, alphanumerics only.
    #[must_use]
    pub fn dedupe_key(&self) -> String {
        self.message
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_lowercase()
    }
}
