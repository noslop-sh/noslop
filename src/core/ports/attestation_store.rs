//! Attestation store port
//!
//! Defines the interface for staging and persisting attestations.

use crate::models::Attestation;

/// Storage backend for attestations
///
/// Implementations handle how attestations are staged during development
/// and persisted in commits (e.g., via trailers, git notes, files).
pub trait AttestationStore: Send + Sync {
    /// Stage an attestation (pending until commit)
    ///
    /// Staged attestations are temporary and will be cleared after commit.
    fn stage(&self, attestation: &Attestation) -> anyhow::Result<()>;

    /// Get all staged attestations
    fn staged(&self) -> anyhow::Result<Vec<Attestation>>;

    /// Clear staged attestations (called after commit succeeds)
    fn clear_staged(&self) -> anyhow::Result<()>;

    /// Format attestations for commit message trailer
    ///
    /// Returns a string suitable for appending to a commit message.
    fn format_trailers(&self, attestations: &[Attestation]) -> String;

    /// Parse attestations from a commit message
    ///
    /// Used to retrieve attestation history from past commits.
    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Attestation>>;
}

/// Storage backend type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageBackend {
    /// Commit message trailers (default, most portable)
    #[default]
    Trailer,
    /// JSON files in .noslop/
    File,
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trailer" | "trailers" => Ok(Self::Trailer),
            "file" | "files" => Ok(Self::File),
            _ => Err(format!("Unknown backend: {s}. Use 'trailer' or 'file'")),
        }
    }
}

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trailer => write!(f, "trailer"),
            Self::File => write!(f, "file"),
        }
    }
}
