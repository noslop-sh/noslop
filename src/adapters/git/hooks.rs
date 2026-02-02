//! Git hooks installation
//!
//! This module handles installation of git hooks for noslop:
//! - pre-commit: Validates assertions are attested
//! - commit-msg: Adds attestation trailers to commit message
//! - post-commit: Clears staged attestations after commit

use std::fs;
use std::path::Path;

/// Install the pre-commit hook
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_pre_commit() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("pre-commit");
    let hook_content = r"#!/bin/sh
# noslop pre-commit hook
# Checks assertions are attested before allowing commit

noslop check
";

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("noslop") {
            return Ok(()); // Already installed
        }
        // Append to existing hook
        let new_content = format!("{}\n\n# noslop\n{}", existing.trim(), hook_content);
        fs::write(&hook_path, new_content)?;
    } else {
        fs::write(&hook_path, hook_content)?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Install the commit-msg hook (used instead of prepare-commit-msg for trailers)
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_commit_msg() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("commit-msg");
    // Need r#"..."# because content contains quotes
    #[allow(clippy::needless_raw_string_hashes)]
    let hook_content = r#"#!/bin/sh
# noslop commit-msg hook
# Adds attestation trailers to commit message

noslop add-trailers "$1"
"#;

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("noslop") {
            return Ok(()); // Already installed
        }
        // Append to existing hook
        let new_content = format!("{}\n\n# noslop\n{}", existing.trim(), hook_content);
        fs::write(&hook_path, new_content)?;
    } else {
        fs::write(&hook_path, hook_content)?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Install the post-commit hook
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_post_commit() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("post-commit");
    let hook_content = r"#!/bin/sh
# noslop post-commit hook
# Clears staged attestations after successful commit

noslop clear-staged
";

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("noslop") {
            return Ok(()); // Already installed
        }
        // Append to existing hook
        let new_content = format!("{}\n\n# noslop\n{}", existing.trim(), hook_content);
        fs::write(&hook_path, new_content)?;
    } else {
        fs::write(&hook_path, hook_content)?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}
