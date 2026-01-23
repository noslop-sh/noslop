//! Centralized path definitions for noslop
//!
//! This module provides a single source of truth for all filesystem paths used by noslop.
//! All path resolution (including git worktree support) is handled here.
//!
//! ## Storage Layout
//!
//! ### Per-Project (Repository Root)
//!
//! ```text
//! repo/                                    # Main worktree
//! ├── .noslop.toml                        # SHARED: Committed config
//! └── .noslop/                            # Local state (gitignored)
//!     ├── refs/tasks/                     # SHARED: Task definitions
//!     │   ├── TSK-1
//!     │   └── TSK-2
//!     └── worktrees/                      # Agent working directories
//!         ├── TSK-1/                      # Agent 1's workspace
//!         │   ├── .noslop/
//!         │   │   └── HEAD               # LOCAL: This agent's task
//!         │   └── src/...
//!         └── TSK-2/                      # Agent 2's workspace
//!             ├── .noslop/
//!             │   └── HEAD
//!             └── src/...
//! ```
//!
//! ### Global (User-Level)
//!
//! ```text
//! ~/.config/noslop/
//! └── config.toml               # User preferences, workspace state
//! ```
//!
//! ## Worktree Support
//!
//! Noslop supports multi-agent workflows via git worktrees:
//! - **Shared state** (main worktree `.noslop/`): Task definitions, config
//! - **Local state** (per-worktree `.noslop/`): HEAD, staged verifications
//!
//! Each agent works in its own worktree with its own HEAD pointer.

use std::path::PathBuf;

use crate::git;

// =============================================================================
// Project-level paths (per-repository)
// =============================================================================

/// Directory name for local noslop state
pub const NOSLOP_DIR: &str = ".noslop";

/// Project configuration filename
pub const NOSLOP_TOML: &str = ".noslop.toml";

/// Current task pointer (like git's HEAD)
const HEAD_FILE: &str = "HEAD";

/// Task refs subdirectory
const REFS_TASKS_DIR: &str = "refs/tasks";

/// Staged verifications filename
const STAGED_VERIFICATIONS_FILE: &str = "staged-verifications.json";

/// Worktrees subdirectory
const WORKTREES_DIR: &str = "worktrees";

/// Get the project root directory.
///
/// For git worktrees, this returns the main worktree root to ensure
/// all worktrees share the same configuration.
#[must_use]
pub fn project_root() -> PathBuf {
    git::get_main_worktree().unwrap_or_else(|| PathBuf::from("."))
}

/// Get path to `.noslop.toml` config file.
///
/// This file is committed to the repository and contains:
/// - Project prefix (e.g., "NOS")
/// - Check definitions
#[must_use]
pub fn noslop_toml() -> PathBuf {
    project_root().join(NOSLOP_TOML)
}

/// Get path to `.noslop/` state directory.
///
/// This directory is gitignored and contains local task state.
#[must_use]
pub fn noslop_dir() -> PathBuf {
    project_root().join(NOSLOP_DIR)
}

/// Get path to `.noslop/HEAD` file.
///
/// Contains the ID of the currently active task.
#[must_use]
pub fn head_file() -> PathBuf {
    noslop_dir().join(HEAD_FILE)
}

/// Get path to `.noslop/refs/tasks/` directory.
///
/// Contains individual task ref files (one JSON file per task).
#[must_use]
pub fn refs_tasks_dir() -> PathBuf {
    noslop_dir().join(REFS_TASKS_DIR)
}

/// Get path to a specific task ref file.
///
/// Each task is stored as `.noslop/refs/tasks/{id}`.
#[must_use]
pub fn task_ref(id: &str) -> PathBuf {
    refs_tasks_dir().join(id)
}

/// Get path to `.noslop/staged-verifications.json`.
///
/// Temporary staging area for verifications before they're added
/// to a commit as trailers.
#[must_use]
pub fn staged_verifications() -> PathBuf {
    noslop_dir().join(STAGED_VERIFICATIONS_FILE)
}

// =============================================================================
// Global paths (user-level)
// =============================================================================

/// Global config directory name
const GLOBAL_DIR: &str = ".noslop";

/// Global config filename
const GLOBAL_CONFIG_FILE: &str = "config.toml";

/// Get the global noslop directory.
///
/// Returns `~/.noslop/`.
#[must_use]
pub fn global_config_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")).join(GLOBAL_DIR)
}

/// Get the global config file path.
///
/// Returns `~/.noslop/config.toml`.
/// Contains user preferences (UI theme, branch visibility).
#[must_use]
pub fn global_config() -> PathBuf {
    global_config_dir().join(GLOBAL_CONFIG_FILE)
}

/// Get the current topic file path.
///
/// Returns `.noslop/current-topic` (local, gitignored).
/// Contains the ID of the currently selected topic.
#[must_use]
pub fn current_topic_file() -> PathBuf {
    noslop_dir().join("current-topic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_structure() {
        // Just verify the path components are correct
        let toml = noslop_toml();
        assert!(toml.ends_with(".noslop.toml"));

        let dir = noslop_dir();
        assert!(dir.ends_with(".noslop"));

        let head = head_file();
        assert!(head.ends_with(".noslop/HEAD") || head.ends_with(".noslop\\HEAD"));

        let refs = refs_tasks_dir();
        assert!(refs.to_string_lossy().contains("refs"));
        assert!(refs.to_string_lossy().contains("tasks"));

        let task = task_ref("TSK-1");
        assert!(task.ends_with("TSK-1"));

        let staged = staged_verifications();
        assert!(staged.ends_with("staged-verifications.json"));

        let global = global_config();
        assert!(global.ends_with("config.toml"));
    }
}
