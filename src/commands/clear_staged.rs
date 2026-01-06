//! Clear staged verifications
//!
//! This command is called by the post-commit hook to remove
//! staged verifications after they've been added to the commit.

use std::path::Path;

/// Clear staged verifications
///
/// Called by post-commit hook to delete .noslop/staged-verifications.json
/// after the commit has been created with verification trailers.
pub fn clear_staged() -> anyhow::Result<()> {
    clear_staged_in(Path::new("."))
}

/// Clear staged verifications in a specific directory (for testing)
fn clear_staged_in(base_dir: &Path) -> anyhow::Result<()> {
    let verifications_file = base_dir.join(".noslop/staged-verifications.json");
    if verifications_file.exists() {
        std::fs::remove_file(verifications_file)?;
    }
    Ok(())
}
