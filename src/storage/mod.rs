//! Storage abstraction for attestations
//!
//! Provides pluggable backends:
//! - `trailer`: Commit message trailers (default, most portable)
//! - `file`: JSON files in .noslop/ (fallback)
//! - `notes`: Git notes (optional, requires config)

#![allow(dead_code)]

/// File-based storage for staging attestations
pub mod file;
/// Commit trailer storage for attestations
pub mod trailer;

use crate::models::Attestation;

/// Storage backend for attestations
pub trait AttestationStore: Send + Sync {
    /// Stage an attestation (pending until commit)
    fn stage(&self, attestation: &Attestation) -> anyhow::Result<()>;

    /// Get all staged attestations
    fn staged(&self) -> anyhow::Result<Vec<Attestation>>;

    /// Clear staged attestations (after commit)
    fn clear_staged(&self) -> anyhow::Result<()>;

    /// Format attestations for commit message trailer
    fn format_trailers(&self, attestations: &[Attestation]) -> String;

    /// Parse attestations from commit message
    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Attestation>>;
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

/// Get the configured attestation store
#[must_use]
pub fn attestation_store() -> Box<dyn AttestationStore> {
    // For now, always use file for staging + trailer for finalized
    // Config-based selection can come later
    Box::new(TrailerAttestationStore::new())
}

// Re-export implementations for direct use
pub use trailer::TrailerAttestationStore;
