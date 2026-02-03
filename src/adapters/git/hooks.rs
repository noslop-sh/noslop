//! Git hooks installation
//!
//! This module handles installation of git hooks for noslop:
//! - pre-commit: Validates checks are acknowledged
//! - commit-msg: Adds acknowledgment trailers to commit message
//! - post-commit: Clears staged acknowledgments after commit

use std::fs;
use std::path::Path;

/// Hook names managed by noslop
const HOOK_NAMES: [&str; 3] = ["pre-commit", "commit-msg", "post-commit"];

/// Remove noslop content from all hooks
///
/// This removes noslop-specific content from hooks while preserving
/// any other hook content that may exist.
///
/// # Errors
///
/// Returns an error if hooks cannot be read or written.
pub fn remove_noslop_hooks() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Ok(()); // No hooks directory, nothing to remove
    }

    for hook_name in HOOK_NAMES {
        let hook_path = hooks_dir.join(hook_name);
        if hook_path.exists() {
            remove_noslop_from_hook(&hook_path)?;
        }
    }

    Ok(())
}

/// Remove noslop content from a single hook file
fn remove_noslop_from_hook(hook_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(hook_path)?;

    // Check if this is a noslop-only hook (starts with noslop shebang comment)
    let is_noslop_only = content
        .lines()
        .take(3)
        .any(|line| line.contains("# noslop") && !line.contains("# noslop\n"))
        && content.lines().filter(|l| l.starts_with('#') && !l.starts_with("#!")).count() <= 2;

    // If the hook only contains noslop content, remove the entire file
    if is_noslop_only
        || content.lines().all(|l| {
            l.trim().is_empty()
                || l.contains("noslop")
                || l.starts_with("#!")
                || l.starts_with("# noslop")
        })
    {
        fs::remove_file(hook_path)?;
        return Ok(());
    }

    // Otherwise, remove just the noslop sections
    let cleaned = remove_noslop_sections(&content);
    if cleaned.trim().is_empty()
        || cleaned.lines().all(|l| l.trim().is_empty() || l.starts_with("#!"))
    {
        fs::remove_file(hook_path)?;
    } else {
        fs::write(hook_path, cleaned)?;
    }

    Ok(())
}

/// Remove noslop sections from hook content while preserving other content
#[must_use]
pub fn remove_noslop_sections(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Start of noslop section (comment containing "noslop")
        if line.trim().starts_with('#') && line.to_lowercase().contains("noslop") {
            // Skip all lines until we hit a new section or end
            i += 1;
            while i < lines.len() {
                let next = lines[i];
                // Empty line - check if next non-empty line is a new section
                if next.trim().is_empty() {
                    // Look ahead for a new section comment
                    let mut j = i + 1;
                    while j < lines.len() && lines[j].trim().is_empty() {
                        j += 1;
                    }
                    if j < lines.len() {
                        let after_empty = lines[j];
                        // New section starts with a comment that doesn't mention noslop
                        if after_empty.trim().starts_with('#')
                            && !after_empty.to_lowercase().contains("noslop")
                        {
                            // End of noslop section, keep the empty line
                            result.push("");
                            i = j;
                            break;
                        }
                    }
                    i += 1;
                    continue;
                }
                // Skip noslop command lines
                if next.contains("noslop") {
                    i += 1;
                    continue;
                }
                // Skip noslop header comment lines
                if next.trim().starts_with('#') && !next.trim().starts_with("#!") {
                    i += 1;
                    continue;
                }
                // Non-comment, non-noslop line - end of noslop section
                break;
            }
            continue;
        }

        result.push(line);
        i += 1;
    }

    // Clean up trailing empty lines
    while result.last().is_some_and(|l| l.trim().is_empty()) {
        result.pop();
    }

    if result.is_empty() {
        String::new()
    } else {
        result.join("\n") + "\n"
    }
}

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
# Checks that checks are acknowledged before allowing commit

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
# Adds acknowledgment trailers to commit message

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
# Clears staged acknowledgments after successful commit

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
