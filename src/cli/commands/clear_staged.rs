//! Clear staged acknowledgments
//!
//! This command is called by the post-commit hook to remove
//! staged acknowledgments after they've been added to the commit.

use std::path::Path;

/// Clear staged acknowledgments
///
/// Called by post-commit hook to delete .noslop/staged-acks.json
/// after the commit has been created with acknowledgment trailers.
pub fn clear_staged() -> anyhow::Result<()> {
    clear_staged_in(Path::new("."))
}

/// Clear staged acknowledgments in a specific directory (for testing)
fn clear_staged_in(base_dir: &Path) -> anyhow::Result<()> {
    let acks_file = base_dir.join(".noslop/staged-acks.json");
    if acks_file.exists() {
        std::fs::remove_file(acks_file)?;
    }
    Ok(())
}
