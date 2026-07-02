//! Check model
//!
//! A check declares: "When this code changes, this must be considered."
//! Think of it as a review-time test that requires human/LLM acknowledgment.

use serde::{Deserialize, Serialize};

use super::Severity;

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
    ///
    /// Checks without an explicit ID get one derived from their content, so
    /// the same check resolves to the same ID on every load (acknowledgments
    /// match by exact ID and would otherwise never resolve).
    #[must_use]
    pub fn new(id: Option<String>, target: String, message: String, severity: Severity) -> Self {
        let id = id.unwrap_or_else(|| derive_id(&target, &message));
        Self {
            id,
            target,
            message,
            severity,
            introduced_by: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Stable content-derived fallback ID (FNV-1a over target + message)
fn derive_id(target: &str, message: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in target.bytes().chain(message.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("c{hash:012x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_id_is_stable_across_loads() {
        let a = Check::new(None, "src/*.rs".into(), "Check the thing".into(), Severity::Block);
        let b = Check::new(None, "src/*.rs".into(), "Check the thing".into(), Severity::Block);
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn different_checks_get_different_ids() {
        let a = Check::new(None, "src/*.rs".into(), "Check A".into(), Severity::Block);
        let b = Check::new(None, "src/*.rs".into(), "Check B".into(), Severity::Block);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn explicit_id_wins() {
        let c = Check::new(Some("NOS-1".into()), "x".into(), "y".into(), Severity::Warn);
        assert_eq!(c.id, "NOS-1");
    }
}
