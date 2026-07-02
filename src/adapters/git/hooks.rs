//! Git hooks installation
//!
//! This module handles installation of git hooks for noslop:
//! - pre-commit: Validates checks are acknowledged
//! - commit-msg: Adds acknowledgment trailers to commit message
//! - post-commit: Clears staged acknowledgments after commit

use std::fs;
use std::path::Path;

/// Install a hook script, appending to any existing non-noslop hook
fn install_hook(name: &str, content: &str) -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join(name);
    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("noslop") {
            return Ok(()); // Already installed
        }
        fs::write(&hook_path, format!("{}\n\n# noslop\n{}", existing.trim(), content))?;
    } else {
        fs::write(&hook_path, content)?;
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

/// Install the pre-commit hook
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_pre_commit() -> anyhow::Result<()> {
    install_hook(
        "pre-commit",
        "#!/bin/sh\n# noslop pre-commit hook\n# Checks that checks are acknowledged before allowing commit\n\nnoslop check\n",
    )
}

/// Install the commit-msg hook (used instead of prepare-commit-msg for trailers)
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_commit_msg() -> anyhow::Result<()> {
    install_hook(
        "commit-msg",
        "#!/bin/sh\n# noslop commit-msg hook\n# Adds acknowledgment trailers to commit message\n\nnoslop add-trailers \"$1\"\n",
    )
}

/// Install the post-commit hook
///
/// # Errors
///
/// Returns an error if not in a git repository or the hook cannot be written.
pub fn install_post_commit() -> anyhow::Result<()> {
    install_hook(
        "post-commit",
        "#!/bin/sh\n# noslop post-commit hook\n# Clears staged acknowledgments after successful commit\n\nnoslop clear-staged\n",
    )
}
