//! Assertion severity levels
//!
//! Defines how strictly an assertion should be enforced.

use serde::{Deserialize, Serialize};

/// Assertion severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational - shown but doesn't block
    Info,
    /// Warning - shown prominently, doesn't block
    Warn,
    /// Blocking - must be attested before commit
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
