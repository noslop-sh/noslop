//! File-based storage for staging verifications
//!
//! Used for staging before commit. Files live in .noslop/
//!
//! WORKTREE SUPPORT:
//! When running from a git worktree, paths are resolved relative to
//! the MAIN worktree to ensure all worktrees share the same state.

use std::fs;

use crate::models::Verification;
use crate::paths;

/// File-based storage (used for staging)
#[derive(Debug, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    /// Load staged verifications from file
    pub fn load_staged_verifications() -> anyhow::Result<Vec<Verification>> {
        let path = paths::staged_verifications();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged verifications to file
    pub fn save_staged_verifications(verifications: &[Verification]) -> anyhow::Result<()> {
        let path = paths::staged_verifications();
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(verifications)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Clear staged verifications
    pub fn clear_staged_verifications() -> anyhow::Result<()> {
        let path = paths::staged_verifications();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}
