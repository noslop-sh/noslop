//! Staged file detection
//!
//! Provides utilities for detecting files staged for commit.

use std::process::Command;

/// Get list of staged files
///
/// # Errors
///
/// Returns an error if git command fails.
pub fn get_staged_files() -> anyhow::Result<Vec<String>> {
    let output = Command::new("git").args(["diff", "--cached", "--name-only"]).output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get staged files");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(String::from).filter(|s| !s.is_empty()).collect())
}
