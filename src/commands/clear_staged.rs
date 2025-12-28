//! Clear staged attestations
//!
//! This command is called by the post-commit hook to remove
//! staged attestations after they've been added to the commit.

use std::path::Path;

/// Clear staged attestations
///
/// Called by post-commit hook to delete .noslop/staged-attestations.json
/// after the commit has been created with attestation trailers.
pub fn clear_staged() -> anyhow::Result<()> {
    clear_staged_in(Path::new("."))
}

/// Clear staged attestations in a specific directory (for testing)
fn clear_staged_in(base_dir: &Path) -> anyhow::Result<()> {
    let attestations_file = base_dir.join(".noslop/staged-attestations.json");
    if attestations_file.exists() {
        std::fs::remove_file(attestations_file)?;
    }
    Ok(())
}
