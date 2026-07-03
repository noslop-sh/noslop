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

/// Fingerprint of the staged state: the index written as a tree object.
///
/// Identical staged content always yields the same oid, so re-running
/// `noslop check` without changes is detectable, and an ack carrying the
/// same oid as the fire event means nothing was changed in between.
///
/// # Errors
///
/// Returns an error if git command fails.
pub fn staged_tree_oid() -> anyhow::Result<String> {
    let output = Command::new("git").args(["write-tree"]).output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to write staged tree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// All tracked files in the repository (`git ls-files`).
///
/// # Errors
///
/// Returns an error if git command fails.
pub fn tracked_files() -> anyhow::Result<Vec<String>> {
    let output = Command::new("git").args(["ls-files"]).output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to list tracked files");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(String::from).filter(|s| !s.is_empty()).collect())
}
