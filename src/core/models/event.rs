//! Check fire event model
//!
//! A fire event records that a check surfaced (blocked or warned) for a
//! staged change. Events are the raw material for `noslop stats`: joined
//! with acknowledgments by check id and staged-tree fingerprint, they
//! distinguish action rate from answers that change nothing.

use serde::{Deserialize, Serialize};

use super::severity::Severity;

/// Version of the on-disk event line format
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// A check firing on a staged change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFireEvent {
    /// Event format version
    pub schema: u32,

    /// The check that fired
    pub check_id: String,

    /// File that matched the check target
    pub file: String,

    /// Severity at fire time
    pub severity: Severity,

    /// Who was committing (detected actor)
    pub actor: String,

    /// Staged-tree object id (`git write-tree`) when the check fired
    pub tree_oid: String,

    /// When the check fired (RFC 3339)
    pub created_at: String,

    /// Agent session's cumulative token count at fire time, when the
    /// agent exposes one (additive, schema 1; see `adapters::agent_spend`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_at_fire: Option<u64>,
}

impl CheckFireEvent {
    /// Create a new fire event stamped with the current time
    #[must_use]
    pub fn new(
        check_id: String,
        file: String,
        severity: Severity,
        actor: String,
        tree_oid: String,
    ) -> Self {
        Self {
            schema: EVENT_SCHEMA_VERSION,
            check_id,
            file,
            severity,
            actor,
            tree_oid,
            created_at: chrono::Utc::now().to_rfc3339(),
            tokens_at_fire: None,
        }
    }

    /// Attach the session's cumulative token count at fire time
    #[must_use]
    pub const fn with_tokens_at_fire(mut self, tokens: Option<u64>) -> Self {
        self.tokens_at_fire = tokens;
        self
    }
}
