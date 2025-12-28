//! File-based storage for staging attestations
//!
//! Used for staging before commit. Files live in .noslop/

use std::fs;
use std::path::Path;

use crate::models::Attestation;

const STAGED_ATTESTATIONS_PATH: &str = ".noslop/staged-attestations.json";

/// File-based storage (used for staging)
pub struct FileStore;

impl FileStore {
    /// Load staged attestations from file
    pub fn load_staged_attestations() -> anyhow::Result<Vec<Attestation>> {
        let path = Path::new(STAGED_ATTESTATIONS_PATH);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged attestations to file
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
    pub fn clear_staged_attestations() -> anyhow::Result<()> {
        let path = Path::new(STAGED_ATTESTATIONS_PATH);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
