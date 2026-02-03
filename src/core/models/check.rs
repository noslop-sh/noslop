//! Check model
//!
//! A check declares: "When this code changes, this must be considered."
//! Think of it as a review-time test that requires human/LLM acknowledgment.

use serde::{Deserialize, Serialize};

use super::{Severity, Target};

/// A check attached to a file or pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Unique identifier (generated)
    pub id: String,

    /// Target path or pattern (e.g., "src/auth.rs" or "src/**/*.rs")
    pub target: String,

    /// The check message - what must be considered
    pub message: String,

    /// Severity: "info", "warn", "block"
    pub severity: Severity,

    /// Commit SHA that introduced this check
    pub introduced_by: Option<String>,

    /// When this check was created
    pub created_at: String,
}

impl Check {
    /// Create a new check with optional custom ID (from TOML)
    pub fn new(id: Option<String>, target: String, message: String, severity: Severity) -> Self {
        Self {
            id: id.unwrap_or_else(generate_id),
            target,
            message,
            severity,
            introduced_by: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Check if this check applies to a given file path
    ///
    /// Supports exact paths and glob patterns via the Target model.
    #[must_use]
    pub fn applies_to(&self, path: &str) -> bool {
        // Parse target as a Target to get full glob support
        // Fall back to simple string matching if parsing fails
        Target::parse(&self.target).map_or_else(
            |_| path == self.target || path.starts_with(&self.target),
            |target| target.matches(path),
        )
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("c{ts:x}")
}
