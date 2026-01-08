//! File-based storage for staging verifications
//!
//! Used for staging before commit. Files live in .noslop/

use std::fs;
use std::path::Path;

use crate::models::Verification;

const STAGED_VERIFICATIONS_PATH: &str = ".noslop/staged-verifications.json";

/// File-based storage (used for staging)
#[derive(Debug, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    /// Load staged verifications from file
    pub fn load_staged_verifications() -> anyhow::Result<Vec<Verification>> {
        let path = Path::new(STAGED_VERIFICATIONS_PATH);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged verifications to file
    pub fn save_staged_verifications(verifications: &[Verification]) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = Path::new(STAGED_VERIFICATIONS_PATH).parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(verifications)?;
        fs::write(STAGED_VERIFICATIONS_PATH, content)?;
        Ok(())
    }

    /// Clear staged verifications
    pub fn clear_staged_verifications() -> anyhow::Result<()> {
        let path = Path::new(STAGED_VERIFICATIONS_PATH);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
