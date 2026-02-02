//! File-based attestation storage
//!
//! Used for staging attestations before commit.
//! Files live in `.noslop/` directory.

use std::fs;
use std::path::Path;

use crate::core::models::Attestation;

const STAGED_ATTESTATIONS_PATH: &str = ".noslop/staged-attestations.json";

/// File-based storage for staging attestations
#[derive(Debug, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    /// Load staged attestations from file
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_staged_attestations() -> anyhow::Result<Vec<Attestation>> {
        let path = Path::new(STAGED_ATTESTATIONS_PATH);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged attestations to file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_staged_attestations(attestations: &[Attestation]) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = Path::new(STAGED_ATTESTATIONS_PATH).parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(attestations)?;
        fs::write(STAGED_ATTESTATIONS_PATH, content)?;
        Ok(())
    }

    /// Clear staged attestations
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be deleted.
    pub fn clear_staged_attestations() -> anyhow::Result<()> {
        let path = Path::new(STAGED_ATTESTATIONS_PATH);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
