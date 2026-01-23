# Worktree-Based Multi-Agent Implementation Specification

## Overview

This document specifies the implementation of worktree-based multi-agent support for noslop. The core idea is:

**One Task = One Agent = One Git Worktree**

Each AI agent (Claude, Codex, etc.) works in its own git worktree, with its own branch, isolated from other agents. Tasks are claimed to prevent double-assignment. The agent CLI process is managed via tmux sessions.

## Goals

1. Enable multiple AI agents to work on different tasks simultaneously
2. Prevent task conflicts through a claiming mechanism
3. Maintain convention enforcement (checks still run per-commit in each worktree)
4. Provide visibility into all active agents and their tasks
5. Support configurable agent backends (claude, codex, cursor, etc.)

## Storage Layout

```
repo/                                    # Main worktree
├── .noslop.toml                        # SHARED: Committed config
│                                       #   - checks, topics, prefix
│                                       #   - [agent] config section
│
└── .noslop/                            # Local state (gitignored)
    │
    ├── HEAD                            # Main worktree's current task (optional)
    │                                   # Most users will use agent worktrees instead
    │
    ├── refs/tasks/                     # SHARED: Task definitions (JSON files)
    │   ├── TSK-1                       # Each task includes claimed_by field
    │   ├── TSK-2
    │   └── TSK-3
    │
    └── worktrees/                      # Agent working directories
        │
        ├── TSK-1/                      # Agent 1's isolated workspace
        │   ├── .noslop/
        │   │   ├── HEAD               # Contains "TSK-1"
        │   │   └── staged-verifications.json
        │   ├── src/
        │   └── ... (full repo copy)
        │
        └── TSK-2/                      # Agent 2's isolated workspace
            ├── .noslop/
            │   └── HEAD               # Contains "TSK-2"
            └── ...
```

### Key Insight: Shared vs Local State

| Path | Scope | Contents |
|------|-------|----------|
| `.noslop.toml` | Shared (committed) | Checks, topics, prefix, agent config |
| `.noslop/refs/tasks/` | Shared (main worktree) | Task definitions, visible to all agents |
| `.noslop/worktrees/` | Shared (main worktree) | Contains all agent worktrees |
| `.noslop/HEAD` | **Per-worktree** | Current task for THIS agent only |
| `.noslop/staged-verifications.json` | **Per-worktree** | Pending verifications for THIS agent |
| `.noslop/current-topic` | **Per-worktree** | Selected topic for THIS agent |

---

## File Changes

### 1. `src/paths.rs`

#### New Functions to Add

```rust
/// Get the current worktree root directory.
///
/// Unlike `project_root()` which always returns the main worktree,
/// this returns the root of whatever worktree we're currently in.
///
/// For the main worktree, this equals `project_root()`.
/// For agent worktrees, this returns `.noslop/worktrees/{task-id}/`.
#[must_use]
pub fn worktree_root() -> PathBuf {
    git::get_current_worktree().unwrap_or_else(|| PathBuf::from("."))
}

/// Get the worktree-local `.noslop/` directory.
///
/// This is where per-agent state lives (HEAD, staged verifications).
/// Each worktree has its own copy of this directory.
#[must_use]
pub fn worktree_local_dir() -> PathBuf {
    worktree_root().join(NOSLOP_DIR)
}

/// Get path to the worktrees directory.
///
/// Returns `.noslop/worktrees/` in the main worktree.
/// This is where all agent worktrees are created.
#[must_use]
pub fn worktrees_dir() -> PathBuf {
    noslop_dir().join("worktrees")
}

/// Get path to a specific agent's worktree directory.
///
/// Returns `.noslop/worktrees/{task_id}/`.
#[must_use]
pub fn agent_worktree(task_id: &str) -> PathBuf {
    worktrees_dir().join(task_id)
}
```

#### Functions to Modify

Change these to use `worktree_local_dir()` instead of `noslop_dir()`:

```rust
/// Get path to `.noslop/HEAD` file.
///
/// CHANGED: Now returns the HEAD for the CURRENT worktree.
/// Each agent worktree has its own HEAD pointing to its task.
#[must_use]
pub fn head_file() -> PathBuf {
    worktree_local_dir().join(HEAD_FILE)  // Was: noslop_dir().join(HEAD_FILE)
}

/// Get path to staged verifications file.
///
/// CHANGED: Now per-worktree so each agent has its own staging area.
#[must_use]
pub fn staged_verifications() -> PathBuf {
    worktree_local_dir().join(STAGED_VERIFICATIONS_FILE)
}

/// Get the current topic file path.
///
/// CHANGED: Now per-worktree.
#[must_use]
pub fn current_topic_file() -> PathBuf {
    worktree_local_dir().join("current-topic")
}
```

#### Constants to Add

```rust
/// Worktrees subdirectory name
const WORKTREES_DIR: &str = "worktrees";
```

---

### 2. `src/git/mod.rs`

#### New Struct

```rust
/// Information about a git worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// HEAD commit hash
    pub head: String,
    /// Branch name (if not detached)
    pub branch: Option<String>,
    /// Whether this is the main worktree
    pub is_main: bool,
}
```

#### New Functions

```rust
/// Get the current worktree root directory.
///
/// Returns the root of whatever worktree we're in (may differ from main).
/// Uses `git rev-parse --show-toplevel`.
#[must_use]
pub fn get_current_worktree() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(PathBuf::from(path))
}

/// List all worktrees in the repository.
///
/// Uses `git worktree list --porcelain` for machine-readable output.
/// Returns worktrees sorted with main worktree first.
pub fn list_worktrees() -> anyhow::Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to list worktrees");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current: Option<WorktreeInfo> = None;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if any
            if let Some(wt) = current.take() {
                worktrees.push(wt);
            }
            // Start new worktree
            let path = PathBuf::from(line.strip_prefix("worktree ").unwrap());
            current = Some(WorktreeInfo {
                path,
                head: String::new(),
                branch: None,
                is_main: false,
            });
        } else if line.starts_with("HEAD ") {
            if let Some(ref mut wt) = current {
                wt.head = line.strip_prefix("HEAD ").unwrap().to_string();
            }
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current {
                // branch refs/heads/main -> main
                let branch = line.strip_prefix("branch refs/heads/").unwrap_or(
                    line.strip_prefix("branch ").unwrap()
                );
                wt.branch = Some(branch.to_string());
            }
        } else if line == "bare" {
            // Skip bare worktrees
            current = None;
        }
    }

    // Don't forget the last one
    if let Some(wt) = current {
        worktrees.push(wt);
    }

    // Mark the main worktree
    if let Some(main) = get_main_worktree() {
        for wt in &mut worktrees {
            if wt.path == main {
                wt.is_main = true;
                break;
            }
        }
    }

    Ok(worktrees)
}

/// Create a new worktree at the given path with a new branch.
///
/// Equivalent to: `git worktree add <path> -b <branch>`
pub fn create_worktree(path: &Path, branch: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["worktree", "add", &path.to_string_lossy(), "-b", branch])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    Ok(())
}

/// Create a new worktree from an existing branch.
///
/// Equivalent to: `git worktree add <path> <branch>`
pub fn create_worktree_from_branch(path: &Path, branch: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["worktree", "add", &path.to_string_lossy(), branch])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    Ok(())
}

/// Remove a worktree.
///
/// Equivalent to: `git worktree remove <path>`
/// Use `force` to remove even with uncommitted changes.
pub fn remove_worktree(path: &Path, force: bool) -> anyhow::Result<()> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path.to_string_lossy());

    let output = Command::new("git").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to remove worktree: {}", stderr);
    }

    Ok(())
}
```

---

### 3. `src/storage/refs.rs`

#### Update TaskRef Struct

Add new field:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TaskRef {
    // ... existing fields ...

    /// Worktree path that has claimed this task.
    /// Set when an agent starts working on the task.
    /// Prevents multiple agents from working on the same task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
}
```

Update the helper struct for deserialization:

```rust
#[derive(Deserialize)]
struct TaskRefHelper {
    // ... existing fields ...

    #[serde(default)]
    claimed_by: Option<String>,
}
```

Update the `Deserialize` impl to include the new field.

Update `TaskRef::new()` and `TaskRef::with_topics()`:

```rust
impl TaskRef {
    pub fn new(title: String) -> Self {
        Self {
            // ... existing fields ...
            claimed_by: None,
        }
    }
}
```

#### Update TaskRefs Methods

**Modify `start()`** to claim the task:

```rust
/// Start a task: set to `in_progress`, make current, claim for this worktree.
///
/// Returns `Err` if the task is already claimed by a different worktree.
/// Returns `Ok(true)` if task was found and started.
/// Returns `Ok(false)` if task not found.
pub fn start(id: &str) -> anyhow::Result<bool> {
    let Some(mut task) = Self::get(id)? else {
        return Ok(false);
    };

    let current_worktree = paths::worktree_root()
        .canonicalize()
        .unwrap_or_else(|_| paths::worktree_root())
        .to_string_lossy()
        .to_string();

    // Check if already claimed by another worktree
    if let Some(ref claimed) = task.claimed_by {
        let claimed_canonical = PathBuf::from(claimed)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(claimed))
            .to_string_lossy()
            .to_string();

        if claimed_canonical != current_worktree {
            anyhow::bail!(
                "Task {} is already claimed by another agent at: {}",
                id,
                claimed
            );
        }
    }

    // Claim the task for this worktree
    task.claimed_by = Some(current_worktree.clone());

    // Set status and timestamps
    let now = chrono::Utc::now().to_rfc3339();
    task.status = "in_progress".to_string();
    if task.started_at.is_none() {
        task.started_at = Some(now);
    }

    Self::write_task(id, &task)?;
    Self::set_current(id)?;

    // Auto-link to current branch if task is unlinked
    if task.branch.is_none() {
        if let Some(branch) = Self::get_current_git_branch() {
            Self::link_branch(id, Some(&branch))?;
        }
    }

    Ok(true)
}
```

**Modify `complete()`** to release the claim:

```rust
/// Complete a task: set to done, clear current, release claim.
pub fn complete(id: &str) -> anyhow::Result<bool> {
    let Some(mut task) = Self::get(id)? else {
        return Ok(false);
    };

    let now = chrono::Utc::now().to_rfc3339();
    task.status = "done".to_string();
    if task.completed_at.is_none() {
        task.completed_at = Some(now);
    }

    // Release the claim
    task.claimed_by = None;

    Self::write_task(id, &task)?;

    // Clear current if this was the current task
    if Self::current()?.as_deref() == Some(id) {
        Self::clear_current()?;
    }

    Ok(true)
}
```

**Add new methods:**

```rust
/// Claim a task for this worktree without starting it.
///
/// Useful for reserving a task before creating the worktree.
pub fn claim(id: &str) -> anyhow::Result<bool> {
    let Some(mut task) = Self::get(id)? else {
        return Ok(false);
    };

    let current_worktree = paths::worktree_root()
        .canonicalize()
        .unwrap_or_else(|_| paths::worktree_root())
        .to_string_lossy()
        .to_string();

    if let Some(ref claimed) = task.claimed_by {
        if claimed != &current_worktree {
            anyhow::bail!("Task {} is already claimed by: {}", id, claimed);
        }
        return Ok(true); // Already claimed by us
    }

    task.claimed_by = Some(current_worktree);
    Self::write_task(id, &task)?;
    Ok(true)
}

/// Release claim on a task without completing it.
///
/// Used when an agent is stopped or worktree is removed.
pub fn release(id: &str) -> anyhow::Result<bool> {
    let Some(mut task) = Self::get(id)? else {
        return Ok(false);
    };

    task.claimed_by = None;
    Self::write_task(id, &task)?;
    Ok(true)
}

/// Check if a task is claimed by any worktree.
pub fn is_claimed(id: &str) -> anyhow::Result<bool> {
    Ok(Self::get(id)?.is_some_and(|t| t.claimed_by.is_some()))
}

/// Get the worktree that has claimed a task.
pub fn claimed_by(id: &str) -> anyhow::Result<Option<String>> {
    Ok(Self::get(id)?.and_then(|t| t.claimed_by))
}
```

---

### 4. `src/noslop_file.rs`

#### Add Agent Config Section

```rust
/// A .noslop.toml file structure
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct NoslopFile {
    #[serde(default)]
    pub project: ProjectConfig,

    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,

    #[serde(default, rename = "topic")]
    pub topics: Vec<TopicEntry>,

    /// Agent configuration
    #[serde(default)]
    pub agent: AgentConfig,
}

/// Agent configuration for multi-agent workflows
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Command to run for the AI agent (e.g., "claude", "codex", "cursor")
    pub command: String,

    /// Whether to use tmux for session management
    pub use_tmux: bool,

    /// Additional arguments to pass to the agent command
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            use_tmux: true,
            args: Vec::new(),
        }
    }
}
```

Update `format_noslop_file()` to include agent config:

```rust
fn format_noslop_file(file: &NoslopFile) -> String {
    // ... existing code ...

    // Write agent config if non-default
    if file.agent.command != "claude" || !file.agent.use_tmux || !file.agent.args.is_empty() {
        out.push_str("[agent]\n");
        if file.agent.command != "claude" {
            let _ = writeln!(out, "command = \"{}\"", file.agent.command);
        }
        if !file.agent.use_tmux {
            let _ = writeln!(out, "use_tmux = false");
        }
        if !file.agent.args.is_empty() {
            let _ = writeln!(out, "args = {:?}", file.agent.args);
        }
        out.push('\n');
    }

    out
}
```

---

### 5. `src/commands/agent.rs` (New File)

Create this new file:

```rust
//! Agent command - manage AI agent worktrees
//!
//! Provides commands to spawn, list, and manage AI agents working in
//! isolated git worktrees. Each agent works on one task in its own
//! worktree with its own branch.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, bail};

use noslop::git;
use noslop::noslop_file;
use noslop::output::OutputMode;
use noslop::paths;
use noslop::storage::TaskRefs;

use crate::cli::AgentAction;

/// Handle agent subcommands
pub fn agent_cmd(action: AgentAction, mode: OutputMode) -> anyhow::Result<()> {
    match action {
        AgentAction::Spawn {
            task_id,
            no_tmux,
            foreground,
        } => spawn(&task_id, no_tmux, foreground, mode),
        AgentAction::List => list(mode),
        AgentAction::Kill {
            task_id,
            keep_worktree,
            force,
        } => kill(&task_id, keep_worktree, force, mode),
        AgentAction::Status => status(mode),
    }
}

/// Spawn a new agent for a task
fn spawn(task_id: &str, no_tmux: bool, foreground: bool, mode: OutputMode) -> anyhow::Result<()> {
    // 1. Verify task exists
    let task = TaskRefs::get(task_id)?
        .ok_or_else(|| anyhow!("Task not found: {}", task_id))?;

    // 2. Check if already claimed
    if let Some(ref claimed) = task.claimed_by {
        bail!(
            "Task {} is already claimed by an agent at: {}\n\
             Use 'noslop agent kill {}' to stop that agent first.",
            task_id,
            claimed,
            task_id
        );
    }

    // 3. Check if worktree already exists
    let worktree_path = paths::agent_worktree(task_id);
    if worktree_path.exists() {
        bail!(
            "Worktree already exists at: {}\n\
             Use 'noslop agent kill {} --keep-worktree' to release the task,\n\
             or remove the directory manually.",
            worktree_path.display(),
            task_id
        );
    }

    // 4. Load agent config
    let config = noslop_file::load_or_default();
    let use_tmux = !no_tmux && config.agent.use_tmux;

    // 5. Create worktree directory parent if needed
    if let Some(parent) = worktree_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 6. Create branch name and worktree
    let branch = format!("agent/{}", task_id.to_lowercase());
    git::create_worktree(&worktree_path, &branch)?;

    // 7. Initialize .noslop/ in the new worktree
    let worktree_noslop = worktree_path.join(".noslop");
    fs::create_dir_all(&worktree_noslop)?;

    // 8. Claim the task (write to shared refs from main worktree context)
    TaskRefs::claim(task_id)?;

    // 9. Set HEAD in the new worktree
    let head_path = worktree_noslop.join("HEAD");
    fs::write(&head_path, format!("{}\n", task_id))?;

    // 10. Build the agent command
    let prompt = build_agent_prompt(task_id, &task);
    let agent_cmd = &config.agent.command;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "task_id": task_id,
                "worktree": worktree_path.to_string_lossy(),
                "branch": branch,
                "use_tmux": use_tmux,
            })
        );
    }

    // 11. Launch the agent
    if foreground {
        // Run in foreground - exec into agent
        println!("Starting agent in foreground...");
        println!("Worktree: {}", worktree_path.display());
        println!("Task: {} - {}", task_id, task.title);

        std::env::set_current_dir(&worktree_path)?;

        // Start the task (sets status to in_progress)
        TaskRefs::start(task_id)?;

        let status = Command::new(agent_cmd)
            .args(&config.agent.args)
            .arg("--print")
            .arg(&prompt)
            .status()?;

        if !status.success() {
            bail!("Agent exited with error");
        }
    } else if use_tmux {
        // Run in tmux session
        let session_name = format!("noslop-{}", task_id);

        // Check if session already exists
        let check = Command::new("tmux")
            .args(["has-session", "-t", &session_name])
            .output()?;

        if check.status.success() {
            bail!("Tmux session '{}' already exists", session_name);
        }

        // Build the command to run in tmux
        // First cd to worktree, then run noslop task start, then run agent
        let tmux_cmd = format!(
            "cd '{}' && noslop task start '{}' && {} {} --print '{}'",
            worktree_path.display(),
            task_id,
            agent_cmd,
            config.agent.args.join(" "),
            prompt.replace('\'', "'\\''"), // Escape single quotes
        );

        Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &session_name,
                "-c",
                &worktree_path.to_string_lossy(),
                &tmux_cmd,
            ])
            .output()?;

        if mode != OutputMode::Json {
            println!("Agent spawned for task: {}", task_id);
            println!("  Title:    {}", task.title);
            println!("  Worktree: {}", worktree_path.display());
            println!("  Branch:   {}", branch);
            println!("  Session:  {}", session_name);
            println!();
            println!("To attach to this agent:");
            println!("  tmux attach -t {}", session_name);
            println!();
            println!("To see all agents:");
            println!("  noslop agent list");
        }
    } else {
        // No tmux, just print instructions
        if mode != OutputMode::Json {
            println!("Worktree created for task: {}", task_id);
            println!("  Title:    {}", task.title);
            println!("  Worktree: {}", worktree_path.display());
            println!("  Branch:   {}", branch);
            println!();
            println!("To start working:");
            println!("  cd {}", worktree_path.display());
            println!("  noslop task start {}", task_id);
            println!("  {}", agent_cmd);
        }
    }

    Ok(())
}

/// Build the initial prompt for the agent
fn build_agent_prompt(task_id: &str, task: &noslop::storage::refs::TaskRef) -> String {
    let mut prompt = format!(
        "You are working on task {}.\n\n\
         Title: {}\n\
         Priority: {}\n",
        task_id, task.title, task.priority
    );

    if let Some(ref desc) = task.description {
        prompt.push_str(&format!("\nDescription:\n{}\n", desc));
    }

    if let Some(ref notes) = task.notes {
        prompt.push_str(&format!("\nNotes:\n{}\n", notes));
    }

    if !task.scope.is_empty() {
        prompt.push_str(&format!("\nRelevant files:\n"));
        for pattern in &task.scope {
            prompt.push_str(&format!("  - {}\n", pattern));
        }
    }

    prompt.push_str(
        "\nRun `noslop task current` to see task details.\n\
         Run `noslop task done` when finished.\n"
    );

    prompt
}

/// List all active agents
fn list(mode: OutputMode) -> anyhow::Result<()> {
    let worktrees = git::list_worktrees()?;
    let tasks = TaskRefs::list()?;

    // Find agent worktrees (those in .noslop/worktrees/)
    let worktrees_dir = paths::worktrees_dir();
    let worktrees_dir_str = worktrees_dir.to_string_lossy();

    let mut agents: Vec<_> = Vec::new();

    for wt in &worktrees {
        // Check if this is an agent worktree
        let wt_str = wt.path.to_string_lossy();
        if !wt_str.contains(&*worktrees_dir_str) && !wt.is_main {
            continue;
        }

        // Find the task claimed by this worktree
        let wt_canonical = wt.path.canonicalize().unwrap_or(wt.path.clone());
        let claimed_task = tasks.iter().find(|(_, t)| {
            t.claimed_by.as_ref().map_or(false, |c| {
                PathBuf::from(c).canonicalize().unwrap_or(PathBuf::from(c)) == wt_canonical
            })
        });

        // Check for tmux session
        let task_id = claimed_task.map(|(id, _)| id.as_str());
        let session_name = task_id.map(|id| format!("noslop-{}", id));
        let tmux_running = session_name.as_ref().map_or(false, |s| {
            Command::new("tmux")
                .args(["has-session", "-t", s])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        });

        agents.push((wt, claimed_task, session_name, tmux_running));
    }

    if mode == OutputMode::Json {
        let json_agents: Vec<_> = agents
            .iter()
            .map(|(wt, task, session, tmux_running)| {
                serde_json::json!({
                    "path": wt.path.to_string_lossy(),
                    "branch": wt.branch,
                    "is_main": wt.is_main,
                    "task_id": task.as_ref().map(|(id, _)| id),
                    "task_title": task.as_ref().map(|(_, t)| &t.title),
                    "task_status": task.as_ref().map(|(_, t)| &t.status),
                    "tmux_session": session,
                    "tmux_running": tmux_running,
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "agents": json_agents }));
    } else {
        if agents.is_empty() {
            println!("No active agents");
            println!();
            println!("To spawn an agent:");
            println!("  noslop agent spawn <task-id>");
            return Ok(());
        }

        println!(
            "{:<20} {:<12} {:<40} {}",
            "TASK", "STATUS", "WORKTREE", "TMUX"
        );
        println!("{}", "-".repeat(80));

        for (wt, task, session, tmux_running) in &agents {
            let task_col = task
                .as_ref()
                .map(|(id, _)| id.as_str())
                .unwrap_or("(none)");
            let status_col = task
                .as_ref()
                .map(|(_, t)| t.status.as_str())
                .unwrap_or("-");
            let path_col = wt.path.to_string_lossy();
            let tmux_col = if *tmux_running {
                session.as_deref().unwrap_or("")
            } else {
                "(not running)"
            };

            println!("{:<20} {:<12} {:<40} {}", task_col, status_col, path_col, tmux_col);
        }

        println!();
        println!("To attach to an agent: tmux attach -t <session>");
        println!("To stop an agent:      noslop agent kill <task-id>");
    }

    Ok(())
}

/// Kill an agent (stop tmux, optionally remove worktree)
fn kill(
    task_id: &str,
    keep_worktree: bool,
    force: bool,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let worktree_path = paths::agent_worktree(task_id);
    let session_name = format!("noslop-{}", task_id);

    // Check if task exists
    let task = TaskRefs::get(task_id)?;

    // Kill tmux session if running
    let tmux_killed = Command::new("tmux")
        .args(["kill-session", "-t", &session_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Release the task claim
    if task.is_some() {
        TaskRefs::release(task_id)?;
    }

    // Remove worktree if requested
    let worktree_removed = if !keep_worktree && worktree_path.exists() {
        git::remove_worktree(&worktree_path, force).is_ok()
    } else {
        false
    };

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "task_id": task_id,
                "tmux_killed": tmux_killed,
                "worktree_removed": worktree_removed,
                "task_released": task.is_some(),
            })
        );
    } else {
        println!("Agent stopped: {}", task_id);
        if tmux_killed {
            println!("  Killed tmux session: {}", session_name);
        }
        if task.is_some() {
            println!("  Released task claim");
        }
        if worktree_removed {
            println!("  Removed worktree: {}", worktree_path.display());
        } else if !keep_worktree && worktree_path.exists() {
            println!(
                "  Worktree not removed (has changes?). Use --force to remove anyway."
            );
        }
    }

    Ok(())
}

/// Show status of current agent
fn status(mode: OutputMode) -> anyhow::Result<()> {
    let current_worktree = paths::worktree_root();
    let current_task = TaskRefs::current()?;
    let is_main = !git::is_linked_worktree();

    let task_info = current_task
        .as_ref()
        .and_then(|id| TaskRefs::get(id).ok().flatten().map(|t| (id.clone(), t)));

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "worktree": current_worktree.to_string_lossy(),
                "is_main_worktree": is_main,
                "task_id": current_task,
                "task_title": task_info.as_ref().map(|(_, t)| &t.title),
                "task_status": task_info.as_ref().map(|(_, t)| &t.status),
            })
        );
    } else {
        println!("Worktree: {}", current_worktree.display());
        if is_main {
            println!("  (main worktree)");
        }

        if let Some((id, task)) = task_info {
            println!();
            println!("Current task: {}", id);
            println!("  Title:  {}", task.title);
            println!("  Status: {}", task.status);
        } else {
            println!();
            println!("No active task in this worktree");
        }
    }

    Ok(())
}
```

---

### 6. `src/cli.rs`

Add the Agent subcommand:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...

    /// Manage AI agents working in worktrees
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
pub enum AgentAction {
    /// Spawn a new agent for a task
    Spawn {
        /// Task ID to work on
        task_id: String,

        /// Don't use tmux (just create worktree)
        #[arg(long)]
        no_tmux: bool,

        /// Run agent in foreground (blocks until agent exits)
        #[arg(long, short = 'f')]
        foreground: bool,
    },

    /// List all active agents
    List,

    /// Stop an agent and optionally remove its worktree
    Kill {
        /// Task ID of the agent to stop
        task_id: String,

        /// Keep the worktree after stopping
        #[arg(long)]
        keep_worktree: bool,

        /// Force remove worktree even with uncommitted changes
        #[arg(long)]
        force: bool,
    },

    /// Show status of current agent/worktree
    Status,
}
```

In `main.rs` or wherever commands are dispatched, add:

```rust
Commands::Agent { action } => commands::agent::agent_cmd(action, mode),
```

---

### 7. `src/commands/mod.rs`

Add the new module:

```rust
pub mod agent;
```

---

### 8. `src/commands/status.rs`

Add `--all` flag to show all agents:

```rust
pub fn status_cmd(all: bool, mode: OutputMode) -> anyhow::Result<()> {
    if all {
        // Delegate to agent list
        return crate::commands::agent::list(mode);
    }

    // ... existing status logic ...
}
```

Update CLI:

```rust
/// Show project status
Status {
    /// Show all agents across all worktrees
    #[arg(long, short = 'a')]
    all: bool,
},
```

---

## Testing

### Unit Tests

Add to `src/storage/refs.rs`:

```rust
#[test]
#[serial(cwd)]
fn test_task_claiming() {
    let _temp = setup();

    let id = TaskRefs::create("Test", None).unwrap();

    // Initially unclaimed
    assert!(!TaskRefs::is_claimed(&id).unwrap());

    // Claim it
    TaskRefs::claim(&id).unwrap();
    assert!(TaskRefs::is_claimed(&id).unwrap());

    // Release it
    TaskRefs::release(&id).unwrap();
    assert!(!TaskRefs::is_claimed(&id).unwrap());
}

#[test]
#[serial(cwd)]
fn test_complete_releases_claim() {
    let _temp = setup();

    let id = TaskRefs::create("Test", None).unwrap();
    TaskRefs::claim(&id).unwrap();
    assert!(TaskRefs::is_claimed(&id).unwrap());

    TaskRefs::complete(&id).unwrap();
    assert!(!TaskRefs::is_claimed(&id).unwrap());
}
```

### Integration Tests

Create `tests/integration/agent_test.rs`:

```rust
//! Integration tests for agent worktree management

use std::process::Command;
use tempfile::TempDir;

fn setup_repo() -> TempDir {
    let temp = TempDir::new().unwrap();
    let path = temp.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();

    // Configure git user
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();

    // Create initial commit
    std::fs::write(path.join("README.md"), "# Test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial"])
        .current_dir(path)
        .output()
        .unwrap();

    // Initialize noslop
    Command::new("cargo")
        .args(["run", "--", "init"])
        .current_dir(path)
        .output()
        .unwrap();

    temp
}

#[test]
fn test_agent_spawn_creates_worktree() {
    let temp = setup_repo();
    let path = temp.path();

    // Create a task
    Command::new("cargo")
        .args(["run", "--", "task", "add", "Test task"])
        .current_dir(path)
        .output()
        .unwrap();

    // Spawn agent (no tmux for testing)
    let output = Command::new("cargo")
        .args(["run", "--", "agent", "spawn", "TSK-1", "--no-tmux"])
        .current_dir(path)
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify worktree exists
    let worktree_path = path.join(".noslop/worktrees/TSK-1");
    assert!(worktree_path.exists());
    assert!(worktree_path.join(".noslop/HEAD").exists());
}

#[test]
fn test_agent_list_shows_agents() {
    let temp = setup_repo();
    let path = temp.path();

    // Create and spawn
    Command::new("cargo")
        .args(["run", "--", "task", "add", "Test task"])
        .current_dir(path)
        .output()
        .unwrap();

    Command::new("cargo")
        .args(["run", "--", "agent", "spawn", "TSK-1", "--no-tmux"])
        .current_dir(path)
        .output()
        .unwrap();

    // List agents
    let output = Command::new("cargo")
        .args(["run", "--", "agent", "list", "--json"])
        .current_dir(path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TSK-1"));
}

#[test]
fn test_agent_kill_removes_worktree() {
    let temp = setup_repo();
    let path = temp.path();

    // Create, spawn, then kill
    Command::new("cargo")
        .args(["run", "--", "task", "add", "Test task"])
        .current_dir(path)
        .output()
        .unwrap();

    Command::new("cargo")
        .args(["run", "--", "agent", "spawn", "TSK-1", "--no-tmux"])
        .current_dir(path)
        .output()
        .unwrap();

    let worktree_path = path.join(".noslop/worktrees/TSK-1");
    assert!(worktree_path.exists());

    Command::new("cargo")
        .args(["run", "--", "agent", "kill", "TSK-1"])
        .current_dir(path)
        .output()
        .unwrap();

    assert!(!worktree_path.exists());
}
```

---

## CLI Reference

After implementation, the following commands will be available:

```bash
# Spawn an agent for a task
noslop agent spawn TSK-1              # Creates worktree, starts tmux session
noslop agent spawn TSK-1 --no-tmux    # Creates worktree only
noslop agent spawn TSK-1 --foreground # Runs agent in current terminal

# List all agents
noslop agent list                     # Show all active agents
noslop agent list --json              # JSON output

# Stop an agent
noslop agent kill TSK-1               # Stop tmux, remove worktree, release task
noslop agent kill TSK-1 --keep-worktree  # Keep the worktree
noslop agent kill TSK-1 --force       # Force remove even with uncommitted changes

# Show current agent status
noslop agent status                   # Status of current worktree

# Show all agents from anywhere
noslop status --all                   # Alias for `noslop agent list`
```

---

## Example Workflow

```bash
# 1. Create some tasks
$ noslop task add "Implement OAuth login" -p p0
Created: TSK-1
$ noslop task add "Fix database connection pooling" -p p1
Created: TSK-2
$ noslop task add "Add rate limiting middleware" -p p2
Created: TSK-3

# 2. Spawn agents for high-priority tasks
$ noslop agent spawn TSK-1
Agent spawned for task: TSK-1
  Title:    Implement OAuth login
  Worktree: .noslop/worktrees/TSK-1
  Branch:   agent/tsk-1
  Session:  noslop-TSK-1

To attach to this agent:
  tmux attach -t noslop-TSK-1

$ noslop agent spawn TSK-2
Agent spawned for task: TSK-2
  ...

# 3. See what's running
$ noslop agent list
TASK                 STATUS       WORKTREE                                 TMUX
--------------------------------------------------------------------------------
TSK-1                in_progress  .noslop/worktrees/TSK-1                  noslop-TSK-1
TSK-2                in_progress  .noslop/worktrees/TSK-2                  noslop-TSK-2

# 4. Attach to watch an agent work
$ tmux attach -t noslop-TSK-1

# 5. When an agent finishes (it runs `noslop task done`)
$ noslop agent list
TASK                 STATUS       WORKTREE                                 TMUX
--------------------------------------------------------------------------------
TSK-1                done         .noslop/worktrees/TSK-1                  (not running)
TSK-2                in_progress  .noslop/worktrees/TSK-2                  noslop-TSK-2

# 6. Clean up completed agents
$ noslop agent kill TSK-1
Agent stopped: TSK-1
  Released task claim
  Removed worktree: .noslop/worktrees/TSK-1
```
