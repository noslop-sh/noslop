//! File-based storage for staging verifications
//!
//! Used for staging before commit. Files live in .noslop/
//!
//! WORKTREE SUPPORT:
//! When running from a git worktree, paths are resolved relative to
//! the MAIN worktree to ensure all worktrees share the same state.

use std::fs;
use std::path::PathBuf;

use crate::git;
use crate::models::Verification;

const STAGED_VERIFICATIONS_FILE: &str = "staged-verifications.json";

/// File-based storage (used for staging)
#[derive(Debug, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    /// Get path to staged verifications file in main worktree
    fn staged_verifications_path() -> PathBuf {
        git::get_main_worktree().map_or_else(
            || PathBuf::from(".noslop").join(STAGED_VERIFICATIONS_FILE),
            |root| root.join(".noslop").join(STAGED_VERIFICATIONS_FILE),
        )
    }

    /// Load staged verifications from file
    pub fn load_staged_verifications() -> anyhow::Result<Vec<Verification>> {
        let path = Self::staged_verifications_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save staged verifications to file
    pub fn save_staged_verifications(verifications: &[Verification]) -> anyhow::Result<()> {
        let path = Self::staged_verifications_path();
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
        let path = Self::staged_verifications_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}
