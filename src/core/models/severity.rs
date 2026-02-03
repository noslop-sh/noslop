//! Check severity levels
//!
//! Defines how strictly a check should be enforced.

use serde::{Deserialize, Serialize};

/// Check severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational - shown but doesn't block
    Info,
    /// Warning - shown prominently, doesn't block
    Warn,
    /// Blocking - must be acknowledged before commit
    #[default]
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
