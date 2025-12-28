//! Assertion model
//!
//! An assertion declares: "When this code changes, this must be considered."
//! Think of it as a review-time test that requires human/LLM attestation.

use serde::{Deserialize, Serialize};

/// An assertion attached to a file or pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// Unique identifier (generated)
    pub id: String,

    /// Target path or pattern (e.g., "src/auth.rs" or "src/**/*.rs")
    pub target: String,

    /// The assertion message - what must be considered
    pub message: String,

    /// Severity: "info", "warn", "block"
    pub severity: Severity,

    /// Commit SHA that introduced this assertion
    pub introduced_by: Option<String>,

    /// When this assertion was created
    pub created_at: String,
}

/// Assertion severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational - shown but doesn't block
    Info,
    /// Warning - shown prominently, doesn't block
    Warn,
    /// Blocking - must be attested before commit
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

impl Assertion {
    /// Create a new assertion with optional custom ID (from TOML)
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

    /// Check if this assertion applies to a given file path
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
    format!("a{ts:x}")
}
