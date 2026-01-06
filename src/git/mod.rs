//! Git integration
//!
//! Provides git-native operations:
//! - Hooks installation
//! - Staged file detection
//! - Repository information
//! - Notes for verifications/intents (coming soon)

pub mod hooks;
pub mod staged;

/// Get the repository name from git remote or directory name
pub fn get_repo_name() -> String {
    // Try to get repo name from git remote
    std::process::Command::new("git")
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
