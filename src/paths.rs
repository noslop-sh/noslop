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
//!     └── agents/                         # Agent workspaces (git worktrees)
//!         ├── agent-1/                    # Agent 1
//!         │   ├── .noslop/
//!         │   │   └── HEAD               # LOCAL: agent-1's current task
//!         │   └── src/...
//!         └── agent-2/                    # Agent 2
//!             ├── .noslop/
//!             │   └── HEAD               # LOCAL: agent-2's current task
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
//! ## Multi-Agent Support
//!
//! Noslop supports multiple AI agents working simultaneously:
//! - **Shared state** (main `.noslop/`): Task definitions, config
//! - **Agent-local state** (per-agent `.noslop/`): HEAD, staged verifications
//!
//! Each agent is a git worktree under the hood, but users interact with
//! "agents" not "worktrees". Use `noslop agent spawn` to create agents.

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

/// Agents subdirectory (each agent is a git worktree)
const AGENTS_DIR: &str = "agents";

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
/// Contains the ID of the currently active task for THIS agent.
/// Each agent has its own HEAD, allowing multiple agents to work
/// on different tasks simultaneously.
#[must_use]
pub fn head_file() -> PathBuf {
    agent_local_dir().join(HEAD_FILE)
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
///
/// NOTE: This is per-agent (each agent has its own staging area).
#[must_use]
pub fn staged_verifications() -> PathBuf {
    agent_local_dir().join(STAGED_VERIFICATIONS_FILE)
}

// =============================================================================
// Agent paths (multi-agent support)
// =============================================================================

/// Get the current agent's root directory.
///
/// For the main worktree, this returns the project root.
/// For agent worktrees, this returns the agent's workspace directory
/// (e.g., `.noslop/agents/agent-1/`).
///
/// This is the working directory where the agent's code lives.
#[must_use]
pub fn agent_root() -> PathBuf {
    git::get_current_worktree().unwrap_or_else(|| PathBuf::from("."))
}

/// Get the agent-local `.noslop/` directory.
///
/// Each agent has its own `.noslop/` for local state (HEAD, staged verifications).
/// This ensures agents don't interfere with each other.
#[must_use]
pub fn agent_local_dir() -> PathBuf {
    agent_root().join(NOSLOP_DIR)
}

/// Get path to the agents directory.
///
/// Returns `.noslop/agents/` in the main worktree.
/// This is where all agent workspaces are created.
#[must_use]
pub fn agents_dir() -> PathBuf {
    noslop_dir().join(AGENTS_DIR)
}

/// Get path to a specific agent's workspace directory.
///
/// Returns `.noslop/agents/{name}/`.
#[must_use]
pub fn agent_path(name: &str) -> PathBuf {
    agents_dir().join(name)
}

/// Check if we're running inside an agent workspace (not the main worktree).
#[must_use]
pub fn is_agent() -> bool {
    git::is_linked_worktree()
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
/// Returns `.noslop/current-topic` for the current agent.
/// Each agent can have its own selected topic context.
#[must_use]
pub fn current_topic_file() -> PathBuf {
    agent_local_dir().join("current-topic")
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

    #[test]
    fn test_agent_paths() {
        // Verify agent path structure
        let agents = agents_dir();
        assert!(agents.to_string_lossy().contains(".noslop"));
        assert!(agents.to_string_lossy().contains("agents"));

        let agent = agent_path("agent-1");
        assert!(agent.ends_with("agent-1"));
        assert!(agent.to_string_lossy().contains("agents"));

        let worker = agent_path("my-worker");
        assert!(worker.ends_with("my-worker"));
    }
}
