//! Storage abstraction for acknowledgments
//!
//! Provides pluggable backends:
//! - `trailer`: Commit message trailers (default, most portable)
//! - `file`: JSON files in .noslop/ (fallback)
//!
//! This module re-exports from `crate::adapters` for backwards compatibility.

#![allow(dead_code)]

// Re-export the AcknowledgmentStore trait from ports
pub use crate::core::ports::AcknowledgmentStore;

// Re-export implementations from adapters
pub use crate::adapters::file::FileStore;
pub use crate::adapters::trailer::{TrailerAckStore, append_trailers};

// Keep file and trailer as submodules for backwards compatibility
pub mod file {
    //! File-based storage re-exports
    pub use crate::adapters::file::FileStore;
}

pub mod trailer {
    //! Trailer storage re-exports
    pub use crate::adapters::trailer::{TrailerAckStore, append_trailers};
}

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Commit message trailers (default)
    #[default]
    Trailer,
    /// JSON files in .noslop/
    File,
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trailer" | "trailers" => Ok(Self::Trailer),
            "file" | "files" => Ok(Self::File),
            _ => Err(format!("Unknown backend: {s}. Use 'trailer' or 'file'")),
        }
    }
}

/// Get the configured acknowledgment store
#[must_use]
pub fn ack_store() -> Box<dyn AcknowledgmentStore> {
    // For now, always use file for staging + trailer for finalized
    // Config-based selection can come later
    Box::new(TrailerAckStore::new())
}
