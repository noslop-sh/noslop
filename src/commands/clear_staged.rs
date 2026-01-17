//! Clear staged verifications and pending task trailers
//!
//! This command is called by the post-commit hook to remove
//! staged verifications and pending task trailers after they've
//! been added to the commit.

use std::path::Path;

use noslop::git;
use noslop::storage::TaskRefs;

/// Clear staged verifications and pending task trailers
///
/// Called by post-commit hook to:
/// 1. Delete .noslop/staged-verifications.json
/// 2. Clear pending_trailer field from all tasks
///
/// This runs after the commit has been created with all trailers.
pub fn clear_staged() -> anyhow::Result<()> {
    // Use main worktree for .noslop/ lookups
    let base_dir = git::get_main_worktree().unwrap_or_else(|| Path::new(".").to_path_buf());
    clear_staged_in(&base_dir)?;

    // Clear pending task trailers
    TaskRefs::clear_all_pending_trailers()?;

    Ok(())
}

/// Clear staged verifications in a specific directory (for testing)
fn clear_staged_in(base_dir: &Path) -> anyhow::Result<()> {
    let verifications_file = base_dir.join(".noslop/staged-verifications.json");
    if verifications_file.exists() {
        std::fs::remove_file(verifications_file)?;
    }
    Ok(())
}
