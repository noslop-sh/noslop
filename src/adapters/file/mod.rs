//! File-based acknowledgment storage
//!
//! Used for staging acknowledgments before commit.
//! Files live in `.noslop/` directory.

use std::fs;
use std::path::Path;

use crate::core::models::Acknowledgment;

const STAGED_ACKS_PATH: &str = ".noslop/staged-acks.json";

/// File-based storage for staging acknowledgments
#[derive(Debug, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    /// Load staged acknowledgments from file
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_staged_acks() -> anyhow::Result<Vec<Acknowledgment>> {
        let path = Path::new(STAGED_ACKS_PATH);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged acknowledgments to file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_staged_acks(acks: &[Acknowledgment]) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = Path::new(STAGED_ACKS_PATH).parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(acks)?;
        fs::write(STAGED_ACKS_PATH, content)?;
        Ok(())
    }

    /// Clear staged acknowledgments
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be deleted.
    pub fn clear_staged_acks() -> anyhow::Result<()> {
        let path = Path::new(STAGED_ACKS_PATH);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
