//! Storage abstraction for verifications and tasks
//!
//! Provides pluggable backends:
//! - `trailer`: Commit message trailers (default, most portable)
//! - `file`: JSON files in .noslop/ (fallback)
//! - `notes`: Git notes (optional, requires config)

#![allow(dead_code)]

/// File-based storage for staging verifications
pub mod file;
/// Task storage
pub mod task;
/// Commit trailer storage for verifications
pub mod trailer;

use crate::models::Verification;

/// Storage backend for verifications
pub trait VerificationStore: Send + Sync {
    /// Stage a verification (pending until commit)
    fn stage(&self, verification: &Verification) -> anyhow::Result<()>;

    /// Get all staged verifications
    fn staged(&self) -> anyhow::Result<Vec<Verification>>;

    /// Clear staged verifications (after commit)
    fn clear_staged(&self) -> anyhow::Result<()>;

    /// Format verifications for commit message trailer
    fn format_trailers(&self, verifications: &[Verification]) -> String;

    /// Parse verifications from commit message
    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Verification>>;
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

/// Get the configured verification store
#[must_use]
pub fn verification_store() -> Box<dyn VerificationStore> {
    // For now, always use file for staging + trailer for finalized
    // Config-based selection can come later
    Box::new(TrailerVerificationStore::new())
}

// Re-export implementations for direct use
#[allow(unused_imports)]
pub use task::TaskStore;
pub use trailer::TrailerVerificationStore;
