//! Acknowledgment model
//!
//! An acknowledgment proves that a check was considered.
//! It's attached to the commit that addresses the check.

use serde::{Deserialize, Serialize};

/// An acknowledgment - proof that a check was considered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acknowledgment {
    /// ID of the check being acknowledged
    pub check_id: String,

    /// The acknowledgment message - how/why it was addressed
    pub message: String,

    /// Who acknowledged: "human", "claude", "gpt-4", etc.
    pub acknowledged_by: String,

    /// When this acknowledgment was made
    pub created_at: String,

    /// Staged-tree object id (`git write-tree`) at ack time.
    ///
    /// Joined against fire events to tell self-correction (tree changed
    /// after the block) from rubber-stamping (acked with nothing changed).
    /// Optional: absent in schema-v1 records written before this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_oid: Option<String>,
}

impl Acknowledgment {
    /// Create a new acknowledgment
    #[must_use]
    pub fn new(check_id: String, message: String, acknowledged_by: String) -> Self {
        Self {
            check_id,
            message,
            acknowledged_by,
            created_at: chrono::Utc::now().to_rfc3339(),
            tree_oid: None,
        }
    }

    /// Create an acknowledgment by the given actor
    #[must_use]
    pub fn by_actor(check_id: String, message: String, actor: &super::Actor) -> Self {
        Self::new(check_id, message, actor.name().to_string())
    }

    /// Attach the staged-tree fingerprint
    #[must_use]
    pub fn with_tree_oid(mut self, tree_oid: Option<String>) -> Self {
        self.tree_oid = tree_oid;
        self
    }
}
