//! Check model
//!
//! A check declares: "When this code changes, this must be verified."
//! Think of it as a review-time gate that requires human/LLM verification.

use serde::{Deserialize, Serialize};

/// A check attached to a file or pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Unique identifier (generated)
    pub id: String,

    /// Target path or pattern (e.g., "src/auth.rs" or "src/**/*.rs")
    pub target: String,

    /// The check message - what must be verified
    pub message: String,

    /// Severity: "info", "warn", "block"
    pub severity: Severity,

    /// Commit SHA that introduced this check
    pub introduced_by: Option<String>,

    /// When this check was created
    pub created_at: String,
}

/// Check severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational - shown but doesn't block
    Info,
    /// Warning - shown prominently, doesn't block
    Warn,
    /// Blocking - must be verified before commit
    Block,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Block => write!(f, "block"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "block" => Ok(Self::Block),
            _ => Err(format!("Invalid severity: {s}. Use: info, warn, block")),
        }
    }
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
    #[must_use]
    #[allow(dead_code)] // Used in tests, will be wired up when Target integration is complete
    pub fn applies_to(&self, path: &str) -> bool {
        // Simple matching for now - exact or prefix
        // TODO: Support glob patterns
        path == self.target || path.starts_with(&self.target) || self.target.contains(path)
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("c{ts:x}")
}
