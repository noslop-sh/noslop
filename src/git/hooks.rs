//! Git hooks installation
//!
//! This module handles installation of git hooks for noslop:
//! - pre-commit: Validates checks are verified
//! - prepare-commit-msg: Adds verification trailers to commit message
//! - post-commit: Clears staged verifications after commit

use std::fs;
use std::path::Path;

/// Install the pre-commit hook
pub fn install_pre_commit() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("pre-commit");
    let hook_content = r"#!/bin/sh
# noslop pre-commit hook
# 1. Validates checks are verified
# 2. Prompts for task status if a task is in progress

noslop check run || exit 1

# Prompt for task status (if a task is active)
noslop task-prompt
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
pub fn install_commit_msg() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("commit-msg");
    let hook_content = r#"#!/bin/sh
# noslop commit-msg hook
# Adds verification trailers to commit message

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
pub fn install_post_commit() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("post-commit");
    let hook_content = r"#!/bin/sh
# noslop post-commit hook
# Clears staged verifications after successful commit

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
