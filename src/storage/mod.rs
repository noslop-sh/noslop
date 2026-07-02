//! Storage abstraction for acknowledgments
//!
//! Acknowledgments are staged in `.noslop/` during development and persisted
//! as commit trailers on commit.

// Re-export the AcknowledgmentStore trait from ports
pub use crate::core::ports::AcknowledgmentStore;

// Re-export implementations from adapters
pub use crate::adapters::file::FileStore;
pub use crate::adapters::trailer::{TrailerAckStore, append_trailers};

/// Get the configured acknowledgment store
#[must_use]
pub fn ack_store() -> Box<dyn AcknowledgmentStore> {
    Box::new(TrailerAckStore::new())
}
