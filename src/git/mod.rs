//! Git integration
//!
//! Provides git-native operations:
//! - Hooks installation
//! - Staged file detection
//! - Repository information
//! - Branch detection
//! - Repository operations (init, checkout, commit)
//! - Notes for verifications/intents (coming soon)

use std::path::Path;
use std::process::Command;

pub mod hooks;
pub mod staged;

/// Get the current git branch name
pub fn get_current_branch() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD") // HEAD means detached state
}

/// Convert branch name to safe filename
/// e.g., "feature/oauth" -> "feature-oauth"
pub fn branch_to_filename(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

/// Get the repository name from git remote or directory name
pub fn get_repo_name() -> String {
    // Try to get repo name from git remote
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            // Extract repo name from URL: https://github.com/user/repo.git -> repo
            s.trim().rsplit('/').next().map(|n| n.trim_end_matches(".git").to_string())
        })
        .or_else(|| {
            // Fallback: use directory name
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        })
        .unwrap_or_else(|| "project".to_string())
}

// =============================================================================
// GIT OPERATIONS (for use in specific directories)
// =============================================================================

/// Initialize a git repository at the given path
pub fn init_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Configure git user for a repository (required for commits)
/// Also disables commit signing to avoid environment-specific failures
pub fn configure_user(path: &Path, email: &str, name: &str) -> bool {
    let email_ok = Command::new("git")
        .args(["config", "user.email", email])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let name_ok = Command::new("git")
        .args(["config", "user.name", name])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Disable commit signing to avoid environment-specific failures
    let gpg_ok = Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    email_ok && name_ok && gpg_ok
}

/// Stage a file in the repository
pub fn add_file(path: &Path, file: &str) -> bool {
    Command::new("git")
        .args(["add", file])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Create a commit with the given message
pub fn commit(path: &Path, message: &str) -> bool {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Disable git hooks in a repository (for test setup)
pub fn disable_hooks(path: &Path) -> bool {
    // Remove or disable hooks by setting core.hooksPath to a non-existent directory
    Command::new("git")
        .args(["config", "core.hooksPath", "/dev/null"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Checkout a branch, optionally creating it
pub fn checkout(path: &Path, branch: &str, create: bool) -> bool {
    let args = if create {
        vec!["checkout", "-b", branch]
    } else {
        vec!["checkout", branch]
    };

    Command::new("git")
        .args(&args)
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get current branch name in a specific directory
pub fn get_branch_in(path: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD")
}
