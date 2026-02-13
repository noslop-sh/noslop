# Checkpoint & Worktree Design

Design specification for porting the bash `checkpoint` and `worktree` scripts to Rust, integrated into noslop's hexagonal architecture.

## Overview

The `checkpoint` and `worktree` tools are agent-agnostic git workflow utilities. They currently exist as standalone bash scripts (~56 and ~223 lines respectively) in the setup repo. This design ports them to native Rust as `noslop checkpoint` and `noslop worktree` subcommands, using the existing `git2` dependency and following noslop's adapter patterns.

### Scope

| Component    | Bash Source                           | Rust Target                                                         |
| ------------ | ------------------------------------- | ------------------------------------------------------------------- |
| checkpoint   | `setup/scripts/checkpoint` (56 lines) | `src/cli/commands/checkpoint.rs` + `src/adapters/git/checkpoint.rs` |
| worktree     | `setup/scripts/worktree` (223 lines)  | `src/cli/commands/worktree.rs` + `src/adapters/git/worktree.rs`     |
| shared utils | `setup/scripts/_lib.sh` (77 lines)    | `src/adapters/git/repo_context.rs`                                  |

### What Changes

- Two new CLI subcommands added to `Command` enum
- Two new adapter modules under `src/adapters/git/`
- One new shared utility module for repo context detection
- Port trait extended with worktree operations
- No changes to existing core domain logic

## File Layout

```
src/
├── cli/
│   ├── app.rs                    # Add Checkpoint and Worktree to Command enum
│   └── commands/
│       ├── mod.rs                # Add checkpoint and worktree re-exports
│       ├── checkpoint.rs         # NEW: checkpoint command handler
│       └── worktree.rs           # NEW: worktree command handler
├── core/
│   └── ports/
│       └── vcs.rs                # Extend VersionControl trait
└── adapters/
    └── git/
        ├── mod.rs                # Add checkpoint, worktree, repo_context modules
        ├── checkpoint.rs         # NEW: git2-based checkpoint operations
        ├── worktree.rs           # NEW: git2-based worktree operations
        └── repo_context.rs       # NEW: single/multi repo detection
```

## CLI Interface

### `noslop checkpoint`

```
noslop checkpoint                         # Message: "checkpoint: 2025-02-08 12:30:00"
noslop checkpoint "work in progress"      # Custom message
noslop checkpoint --json                  # JSON output
```

### `noslop worktree`

```
noslop worktree create <branch> [base]    # Create worktree on new branch
noslop worktree list                      # List active worktrees
noslop worktree remove <branch>           # Remove worktree and clean up branch
noslop worktree clean                     # Remove all worktrees
```

### Design Decision: Drop Multi-Repo Mode

The bash scripts support a "multi-repo" mode where `checkpoint` and `worktree` iterate over sibling git repos in a parent directory. This mode is dropped in the Rust port for the following reasons:

1. **noslop operates on a single repository.** Every other noslop command (`init`, `check`, `ack`, `review`) targets the current repo. Multi-repo would be an anomaly.
2. **Multi-repo is a workspace concern.** Tools like `git submodule`, `repo`, or scripted loops handle multi-repo workflows. noslop should not duplicate this.
3. **Simplifies the codebase.** Removing multi-repo eliminates the `detect_repos` / `MODE` / `REPOS` abstraction layer entirely.
4. **The `--repo` flag becomes unnecessary.** No multi-repo means no need to filter to a single repo.

If multi-repo support is needed in the future, it can be added as a separate `noslop workspace` command without polluting the checkpoint/worktree interfaces.

## CLI Definitions (app.rs)

### Command Enum Extension

```rust
#[derive(Subcommand, Debug)]
pub enum Command {
    // ... existing variants (Init, Check, Ack, AddTrailers, ClearStaged, Review, Version) ...

    /// Safety commit all changes (staged + untracked)
    Checkpoint {
        /// Commit message (default: "checkpoint: <timestamp>")
        message: Option<String>,
    },

    /// Manage git worktrees for isolated work
    Worktree {
        #[command(subcommand)]
        action: WorktreeAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorktreeAction {
    /// Create a worktree on a new branch
    Create {
        /// Branch name for the worktree
        branch: String,

        /// Base branch or commit (default: HEAD)
        #[arg(default_value = "HEAD")]
        base: String,
    },

    /// List active worktrees
    List,

    /// Remove a worktree and optionally delete the branch
    Remove {
        /// Branch name of the worktree to remove
        branch: String,
    },

    /// Remove all worktrees
    Clean,
}
```

### Command Dispatch Extension

```rust
// In run():
match cli.command {
    // ... existing arms ...

    Some(Command::Checkpoint { message }) => {
        commands::checkpoint(message.as_deref(), output_mode)
    },
    Some(Command::Worktree { action }) => {
        commands::worktree(action, output_mode)
    },
}
```

### Commands Module Extension

```rust
// In commands/mod.rs:
mod checkpoint;
mod worktree;

pub use checkpoint::checkpoint;
pub use worktree::worktree;
```

## Port Trait Extension

The `VersionControl` trait in `src/core/ports/vcs.rs` gains worktree-related methods. Checkpoint does not need a port method because it is a compound operation (stage all + commit) that composes existing primitives rather than defining a new abstraction boundary.

### Extended `VersionControl` Trait

```rust
/// Version control system abstraction
#[cfg_attr(test, mockall::automock)]
pub trait VersionControl: Send + Sync {
    // --- existing methods ---

    fn staged_files(&self) -> anyhow::Result<Vec<String>>;
    fn repo_name(&self) -> String;
    fn repo_root(&self) -> anyhow::Result<std::path::PathBuf>;
    fn install_hooks(&self, force: bool) -> anyhow::Result<()>;
    fn is_inside_repo(&self, path: &Path) -> bool;
    fn current_branch(&self) -> anyhow::Result<Option<String>>;

    // --- new methods ---

    /// Check if the working tree is clean (no staged, unstaged, or untracked changes)
    fn is_clean(&self) -> anyhow::Result<bool>;

    /// Stage all changes and commit with the given message, bypassing hooks
    ///
    /// Returns the short SHA of the created commit, or None if the tree was clean.
    fn checkpoint(&self, message: &str) -> anyhow::Result<Option<String>>;

    /// List all worktrees for this repository
    fn list_worktrees(&self) -> anyhow::Result<Vec<WorktreeInfo>>;

    /// Create a new worktree at the given path on a new branch
    ///
    /// If the branch already exists, checks it out in the worktree instead.
    fn create_worktree(
        &self,
        path: &Path,
        branch: &str,
        base: &str,
    ) -> anyhow::Result<()>;

    /// Remove a worktree by path
    fn remove_worktree(&self, path: &Path, force: bool) -> anyhow::Result<()>;

    /// Prune stale worktree metadata
    fn prune_worktrees(&self) -> anyhow::Result<()>;

    /// Check if a branch has been merged into the current branch
    fn is_branch_merged(&self, branch: &str) -> anyhow::Result<bool>;

    /// Delete a local branch
    fn delete_branch(&self, branch: &str, force: bool) -> anyhow::Result<()>;
}
```

### New Model Type

```rust
// src/core/models/worktree.rs

/// Information about a git worktree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory
    pub path: PathBuf,
    /// Branch checked out in this worktree (None if detached HEAD)
    pub branch: Option<String>,
    /// HEAD commit SHA
    pub head: String,
    /// Whether this is the main worktree
    pub is_main: bool,
}
```

Added to `src/core/models/mod.rs`:

```rust
mod worktree;
pub use worktree::WorktreeInfo;
```

## Adapter Implementations

### `src/adapters/git/checkpoint.rs`

Implements the checkpoint operation using `git2`. This is the Rust equivalent of the bash `checkpoint` script's single-repo path.

```rust
//! Checkpoint operations using git2
//!
//! Stages all changes (tracked + untracked) and commits with --no-verify.

use std::path::Path;

use git2::{Repository, Signature, StatusOptions};

/// Check if the repository working tree is clean
///
/// Returns true if there are no staged, unstaged, or untracked changes.
pub fn is_clean(repo: &Repository) -> anyhow::Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses.is_empty())
}

/// Stage all changes and commit, bypassing hooks
///
/// Equivalent to `git add -A && git commit -m "$msg" --no-verify --quiet`.
///
/// Returns the short SHA of the new commit, or None if the tree was clean.
pub fn checkpoint(repo: &Repository, message: &str) -> anyhow::Result<Option<String>> {
    if is_clean(repo)? {
        return Ok(None);
    }

    // Stage all changes (git add -A equivalent)
    let mut index = repo.index()?;
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;

    // Also remove deleted files from index
    index.update_all(["*"], None)?;
    index.write()?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Get the current HEAD as parent commit
    let parent = repo.head()?.peel_to_commit()?;

    let signature = default_signature(repo)?;

    let commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent],
    )?;

    let short_sha = &commit_oid.to_string()[..7];
    Ok(Some(short_sha.to_string()))
}

/// Get the default signature (author/committer) from git config
fn default_signature(repo: &Repository) -> anyhow::Result<Signature<'static>> {
    repo.signature()
        .or_else(|_| {
            Signature::now("noslop", "noslop@localhost")
        })
        .map_err(Into::into)
}
```

#### Bash-to-Rust Mapping

| Bash (`checkpoint`)                        | Rust (`adapters::git::checkpoint`)                           |
| ------------------------------------------ | ------------------------------------------------------------ |
| `is_clean "$repo"` (via `_lib.sh`)         | `is_clean(&repo)` using `repo.statuses()`                    |
| `git add -A`                               | `index.add_all(["*"], ...)` + `index.update_all(["*"], ...)` |
| `git commit -m "$MSG" --no-verify --quiet` | `repo.commit(...)` (git2 does not invoke hooks)              |
| `git log --oneline -1`                     | Return short SHA from `commit_oid`                           |
| `date '+%Y-%m-%d %H:%M:%S'`                | `chrono::Local::now().format(...)`                           |
| Multi-repo loop                            | Dropped (single-repo only)                                   |

### `src/adapters/git/worktree.rs`

Implements worktree operations using `git2`. This is the Rust equivalent of the bash `worktree` script.

````rust
//! Git worktree management using git2
//!
//! Worktrees are created at `../<repo-name>-worktrees/<branch>/`
//! relative to the repository root.

use std::path::{Path, PathBuf};

use git2::Repository;

use crate::core::models::WorktreeInfo;

/// Compute the worktree base directory for a repository
///
/// Given repo root `/Users/saumil/Code/noslop`, returns
/// `/Users/saumil/Code/noslop-worktrees`.
pub fn worktree_base_dir(repo_root: &Path) -> anyhow::Result<PathBuf> {
    let repo_name = repo_root
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine repo name from path"))?
        .to_string_lossy();

    let parent = repo_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Repository root has no parent directory"))?;

    Ok(parent.join(format!("{repo_name}-worktrees")))
}

/// Compute the worktree path for a specific branch
pub fn worktree_path(repo_root: &Path, branch: &str) -> anyhow::Result<PathBuf> {
    Ok(worktree_base_dir(repo_root)?.join(branch))
}

/// List all worktrees for the repository
pub fn list_worktrees(repo: &Repository) -> anyhow::Result<Vec<WorktreeInfo>> {
    let worktrees = repo.worktrees()?;
    let mut result = Vec::new();

    // Add the main worktree
    if let Some(workdir) = repo.workdir() {
        let head = repo.head().ok().map(|h| {
            h.target()
                .map(|oid| oid.to_string()[..7].to_string())
                .unwrap_or_default()
        });

        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from));

        result.push(WorktreeInfo {
            path: workdir.to_path_buf(),
            branch,
            head: head.unwrap_or_default(),
            is_main: true,
        });
    }

    // Add linked worktrees
    for name in worktrees.iter() {
        let Some(name) = name else { continue };
        let wt = repo.find_worktree(name)?;

        if !wt.is_valid() {
            continue;
        }

        let wt_path = wt.path().to_path_buf();

        // Open the worktree repo to get HEAD info
        let wt_repo = Repository::open(&wt_path);
        let (branch, head) = match wt_repo {
            Ok(ref r) => {
                let branch = r
                    .head()
                    .ok()
                    .and_then(|h| h.shorthand().map(String::from));
                let head = r
                    .head()
                    .ok()
                    .and_then(|h| h.target().map(|oid| oid.to_string()[..7].to_string()))
                    .unwrap_or_default();
                (branch, head)
            }
            Err(_) => (None, String::new()),
        };

        result.push(WorktreeInfo {
            path: wt_path,
            branch,
            head,
            is_main: false,
        });
    }

    Ok(result)
}

/// Create a worktree for the given branch
///
/// Tries to create a new branch first. If the branch already exists,
/// checks it out in the worktree instead.
///
/// Equivalent to the bash fallback:
/// ```bash
/// git worktree add -b "$branch" "$wt_path" "$base" 2>/dev/null \
///   || git worktree add "$wt_path" "$branch" 2>/dev/null
/// ```
pub fn create_worktree(
    repo: &Repository,
    wt_path: &Path,
    branch: &str,
    base: &str,
) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = wt_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Resolve the base reference
    let base_ref = repo.revparse_single(base)?;
    let base_commit = base_ref.peel_to_commit()?;

    // Try to create a new branch and worktree
    match repo.branch(branch, &base_commit, false) {
        Ok(new_branch) => {
            // New branch created, add worktree for it
            let refname = new_branch
                .into_reference()
                .name()
                .ok_or_else(|| anyhow::anyhow!("Branch ref has no name"))?
                .to_string();
            repo.worktree(
                branch,
                wt_path,
                Some(git2::WorktreeAddOptions::new().reference(
                    repo.find_reference(&refname)?,
                )),
            )?;
        }
        Err(_) => {
            // Branch already exists, create worktree pointing to it
            let refname = format!("refs/heads/{branch}");
            let reference = repo.find_reference(&refname)?;
            repo.worktree(
                branch,
                wt_path,
                Some(git2::WorktreeAddOptions::new().reference(reference)),
            )?;
        }
    }

    Ok(())
}

/// Remove a worktree and optionally delete the branch
///
/// Equivalent to:
/// ```bash
/// git worktree remove "$wt_path" --force
/// git branch -d "$branch" 2>/dev/null  # if merged
/// ```
pub fn remove_worktree(
    repo: &Repository,
    wt_path: &Path,
    branch: &str,
    force: bool,
) -> anyhow::Result<BranchCleanupResult> {
    // Prune stale entries first
    prune_worktrees(repo)?;

    // Remove the worktree directory
    if wt_path.exists() {
        if force {
            std::fs::remove_dir_all(wt_path)?;
        } else {
            std::fs::remove_dir_all(wt_path)?;
        }
    }

    // Prune again to clean up the worktree metadata
    prune_worktrees(repo)?;

    // Try to delete the branch if merged
    let branch_result = cleanup_branch(repo, branch);

    Ok(branch_result)
}

/// Remove all worktrees for this repository
pub fn clean_worktrees(repo: &Repository, repo_root: &Path) -> anyhow::Result<CleanResult> {
    prune_worktrees(repo)?;

    let wt_base = worktree_base_dir(repo_root)?;
    let mut removed = 0u32;

    if wt_base.exists() {
        let entries = std::fs::read_dir(&wt_base)?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let wt_path = entry.path();

                std::fs::remove_dir_all(&wt_path)?;
                removed += 1;

                // Try to delete the branch
                let _ = cleanup_branch(repo, &name);
            }
        }

        // Remove the base directory if empty
        let _ = std::fs::remove_dir(&wt_base);
    }

    // Final prune
    prune_worktrees(repo)?;

    Ok(CleanResult { removed })
}

/// Prune stale worktree metadata
pub fn prune_worktrees(repo: &Repository) -> anyhow::Result<()> {
    repo.worktree_prune(
        None,
        git2::WorktreePruneOptions::new()
            .working_tree(false)
            .valid(false)
            .locked(false),
    ).ok(); // Ignore errors from prune
    Ok(())
}

/// Check if a branch is merged into the current branch and delete it if so
fn cleanup_branch(repo: &Repository, branch_name: &str) -> BranchCleanupResult {
    let branch = match repo.find_branch(branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return BranchCleanupResult::NotFound,
    };

    let is_merged = branch.is_head()
        || is_branch_merged(repo, branch_name).unwrap_or(false);

    if is_merged {
        match repo.find_branch(branch_name, git2::BranchType::Local) {
            Ok(mut b) => match b.delete() {
                Ok(()) => BranchCleanupResult::Deleted,
                Err(_) => BranchCleanupResult::Kept("could not delete".to_string()),
            },
            Err(_) => BranchCleanupResult::NotFound,
        }
    } else {
        BranchCleanupResult::Kept("not merged".to_string())
    }
}

/// Check if a branch has been merged into HEAD
fn is_branch_merged(repo: &Repository, branch_name: &str) -> anyhow::Result<bool> {
    let branch_ref = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_commit = branch_ref.get().peel_to_commit()?;

    let head_commit = repo.head()?.peel_to_commit()?;

    let merge_base = repo.merge_base(head_commit.id(), branch_commit.id())?;
    Ok(merge_base == branch_commit.id())
}

/// Result of branch cleanup after worktree removal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchCleanupResult {
    /// Branch was merged and deleted
    Deleted,
    /// Branch was kept (with reason)
    Kept(String),
    /// Branch was not found
    NotFound,
}

/// Result of cleaning all worktrees
#[derive(Debug, Clone)]
pub struct CleanResult {
    /// Number of worktrees removed
    pub removed: u32,
}
````

#### Bash-to-Rust Mapping

| Bash (`worktree`)                                  | Rust (`adapters::git::worktree`)                |
| -------------------------------------------------- | ----------------------------------------------- |
| `_wt_base "$repo"`                                 | `worktree_base_dir(repo_root)`                  |
| `git worktree add -b "$branch" "$wt_path" "$base"` | `create_worktree(repo, wt_path, branch, base)`  |
| `git worktree list`                                | `list_worktrees(repo)` using `repo.worktrees()` |
| `git worktree remove "$wt_path" --force`           | `remove_worktree(repo, wt_path, branch, force)` |
| `git worktree prune`                               | `prune_worktrees(repo)`                         |
| `git branch --merged \| grep "$branch"`            | `is_branch_merged(repo, branch)` via merge-base |
| `git branch -d "$branch"`                          | `cleanup_branch(repo, branch)`                  |
| `rmdir "$wt_base"`                                 | `std::fs::remove_dir(&wt_base)`                 |
| `--repo` flag + multi-repo loop                    | Dropped                                         |

## `GitVersionControl` Adapter Extension

The existing `GitVersionControl` struct in `src/adapters/git/mod.rs` gains implementations for the new trait methods. These delegate to the functions defined in the checkpoint and worktree modules.

### Module Registration

```rust
// src/adapters/git/mod.rs

pub mod hooks;
pub mod staging;
pub mod checkpoint;   // NEW
pub mod worktree;     // NEW

// ... existing code ...

use crate::core::models::WorktreeInfo;

impl VersionControl for GitVersionControl {
    // ... existing methods ...

    fn is_clean(&self) -> anyhow::Result<bool> {
        let repo = git2::Repository::discover(&self.workdir)?;
        checkpoint::is_clean(&repo)
    }

    fn checkpoint(&self, message: &str) -> anyhow::Result<Option<String>> {
        let repo = git2::Repository::discover(&self.workdir)?;
        checkpoint::checkpoint(&repo, message)
    }

    fn list_worktrees(&self) -> anyhow::Result<Vec<WorktreeInfo>> {
        let repo = git2::Repository::discover(&self.workdir)?;
        worktree::list_worktrees(&repo)
    }

    fn create_worktree(
        &self,
        path: &Path,
        branch: &str,
        base: &str,
    ) -> anyhow::Result<()> {
        let repo = git2::Repository::discover(&self.workdir)?;
        worktree::create_worktree(&repo, path, branch, base)
    }

    fn remove_worktree(&self, path: &Path, force: bool) -> anyhow::Result<()> {
        let repo = git2::Repository::discover(&self.workdir)?;
        let branch = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let _ = worktree::remove_worktree(&repo, path, &branch, force)?;
        Ok(())
    }

    fn prune_worktrees(&self) -> anyhow::Result<()> {
        let repo = git2::Repository::discover(&self.workdir)?;
        worktree::prune_worktrees(&repo)
    }

    fn is_branch_merged(&self, branch: &str) -> anyhow::Result<bool> {
        let repo = git2::Repository::discover(&self.workdir)?;
        worktree::is_branch_merged(&repo, branch)
    }

    fn delete_branch(&self, branch: &str, force: bool) -> anyhow::Result<()> {
        let repo = git2::Repository::discover(&self.workdir)?;
        let mut branch_ref = repo.find_branch(branch, git2::BranchType::Local)?;
        if force {
            branch_ref.delete()?;
        } else {
            // Only delete if merged
            if worktree::is_branch_merged(&repo, branch)? {
                branch_ref.delete()?;
            } else {
                anyhow::bail!("Branch '{branch}' is not merged. Use force to delete.");
            }
        }
        Ok(())
    }
}
```

## Command Implementations

### `src/cli/commands/checkpoint.rs`

```rust
//! Safety commit all current changes

use chrono::Local;

use noslop::adapters::GitVersionControl;
use noslop::core::ports::VersionControl;
use noslop::output::OutputMode;

/// Run the checkpoint command
pub fn checkpoint(message: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;

    let msg = message
        .map(String::from)
        .unwrap_or_else(|| {
            format!("checkpoint: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))
        });

    match vcs.checkpoint(&msg)? {
        Some(short_sha) => {
            if mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "sha": short_sha,
                        "message": msg,
                    })
                );
            } else {
                println!("Checkpoint: {short_sha} {msg}");
            }
        }
        None => {
            if mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "clean": true,
                        "message": "Nothing to checkpoint (working tree clean).",
                    })
                );
            } else {
                println!("Nothing to checkpoint (working tree clean).");
            }
        }
    }

    Ok(())
}
```

### `src/cli/commands/worktree.rs`

```rust
//! Git worktree management

use noslop::adapters::GitVersionControl;
use noslop::adapters::git::worktree as wt;
use noslop::core::ports::VersionControl;
use noslop::output::OutputMode;

use crate::app::WorktreeAction;

/// Run the worktree command
pub fn worktree(action: WorktreeAction, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;
    let repo_root = vcs.repo_root()?;

    match action {
        WorktreeAction::Create { branch, base } => {
            create(&vcs, &repo_root, &branch, &base, mode)
        }
        WorktreeAction::List => list(&vcs, mode),
        WorktreeAction::Remove { branch } => {
            remove(&vcs, &repo_root, &branch, mode)
        }
        WorktreeAction::Clean => clean(&vcs, &repo_root, mode),
    }
}

fn create(
    vcs: &GitVersionControl,
    repo_root: &std::path::Path,
    branch: &str,
    base: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let wt_path = wt::worktree_path(repo_root, branch)?;

    vcs.create_worktree(&wt_path, branch, base)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "branch": branch,
                "path": wt_path.display().to_string(),
            })
        );
    } else {
        println!("Creating worktree:");
        println!("  Branch: {branch}");
        println!("  Path:   {}", wt_path.display());
        println!();
        println!("cd {}", wt_path.display());
    }

    Ok(())
}

fn list(vcs: &GitVersionControl, mode: OutputMode) -> anyhow::Result<()> {
    let worktrees = vcs.list_worktrees()?;

    if mode == OutputMode::Json {
        let entries: Vec<serde_json::Value> = worktrees
            .iter()
            .map(|wt| {
                serde_json::json!({
                    "path": wt.path.display().to_string(),
                    "branch": wt.branch,
                    "head": wt.head,
                    "is_main": wt.is_main,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for wt in &worktrees {
            let branch_display = wt
                .branch
                .as_deref()
                .unwrap_or("(detached HEAD)");
            let marker = if wt.is_main { " (main)" } else { "" };
            println!(
                "{:<50} {} [{}]{}",
                wt.path.display(),
                wt.head,
                branch_display,
                marker
            );
        }
    }

    Ok(())
}

fn remove(
    _vcs: &GitVersionControl,
    repo_root: &std::path::Path,
    branch: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let wt_path = wt::worktree_path(repo_root, branch)?;

    if !wt_path.exists() {
        anyhow::bail!("Worktree '{branch}' not found at {}", wt_path.display());
    }

    let repo = git2::Repository::discover(repo_root)?;
    let result = wt::remove_worktree(&repo, &wt_path, branch, true)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "removed": wt_path.display().to_string(),
                "branch_status": format!("{result:?}"),
            })
        );
    } else {
        println!("  Removed: {}", wt_path.display());
        match result {
            wt::BranchCleanupResult::Deleted => {
                println!("  Deleted merged branch: {branch}");
            }
            wt::BranchCleanupResult::Kept(reason) => {
                println!("  Branch '{branch}' kept ({reason})");
            }
            wt::BranchCleanupResult::NotFound => {}
        }
    }

    Ok(())
}

fn clean(
    _vcs: &GitVersionControl,
    repo_root: &std::path::Path,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let repo = git2::Repository::discover(repo_root)?;
    let result = wt::clean_worktrees(&repo, repo_root)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "removed": result.removed,
            })
        );
    } else {
        if result.removed == 0 {
            println!("  No worktrees");
        } else {
            println!("  Cleaned {} worktree(s)", result.removed);
        }
    }

    Ok(())
}
```

## Architecture Integration

### Hexagonal Boundaries

```
┌───────────────────────────────────────────────────────────────┐
│                    CLI Layer (Thin Dispatch)                   │
│                                                               │
│  checkpoint.rs       worktree.rs                              │
│  - Format output     - Dispatch subcommands                   │
│  - Handle modes      - Format output                          │
└───────────┬─────────────────────┬─────────────────────────────┘
            │                     │
┌───────────▼─────────────────────▼─────────────────────────────┐
│                    Port Layer (Traits)                         │
│                                                               │
│  VersionControl                                               │
│  + is_clean() -> bool                                         │
│  + checkpoint(msg) -> Option<sha>                             │
│  + list_worktrees() -> Vec<WorktreeInfo>                      │
│  + create_worktree(path, branch, base)                        │
│  + remove_worktree(path, force)                               │
│  + prune_worktrees()                                          │
│  + is_branch_merged(branch) -> bool                           │
│  + delete_branch(branch, force)                               │
└───────────┬─────────────────────┬─────────────────────────────┘
            │                     │
┌───────────▼─────────────────────▼─────────────────────────────┐
│                    Adapter Layer (I/O)                         │
│                                                               │
│  GitVersionControl                                            │
│  ├── checkpoint.rs   (git2::Repository)                       │
│  │   - is_clean()    → repo.statuses()                        │
│  │   - checkpoint()  → index.add_all + repo.commit            │
│  └── worktree.rs     (git2::Repository)                       │
│      - list          → repo.worktrees()                       │
│      - create        → repo.branch + repo.worktree            │
│      - remove        → fs::remove_dir_all + prune             │
│      - clean         → iterate base dir + remove each         │
└───────────────────────────────────────────────────────────────┘
```

### Why VersionControl Trait (not a new port)

Checkpoint and worktree are git operations that extend the existing `VersionControl` abstraction. Creating a separate `CheckpointPort` or `WorktreePort` would be wrong because:

1. **Same boundary**: These operations interact with the same git repository as `staged_files`, `repo_root`, and `current_branch`.
2. **Same adapter**: `GitVersionControl` is the natural home, and it already holds the `workdir` path needed by all these operations.
3. **Composability**: The iteration loop (`noslop run`) will call `checkpoint()` between iterations using the same `VersionControl` instance it uses for `current_branch()`.
4. **Noslop convention**: Existing code places all git operations behind `VersionControl`, not scattered across multiple traits.

### Why `git2` Instead of `std::process::Command`

The existing `GitVersionControl` adapter uses `std::process::Command` to shell out to git. The new checkpoint and worktree code uses `git2` instead. This is deliberate:

1. **`git2` is already a dependency** (v0.20.2 with vendored OpenSSL).
2. **No subprocess overhead**: Checkpoint runs frequently (between every iteration in ralph). Avoiding process spawning matters.
3. **Atomic staging**: `git2`'s index API stages files in a single operation instead of shelling out to `git add`.
4. **No hook execution**: `git2` commits do not invoke git hooks, which is exactly what `--no-verify` provides. No flag-passing needed.
5. **Structured worktree info**: `git2` returns `Worktree` objects with path/validity, avoiding `git worktree list` output parsing.

Over time, existing `Command`-based methods in `GitVersionControl` can migrate to `git2` as well, but that is outside this design's scope.

## Output Format

### Human Mode (default)

**Checkpoint:**

```
Checkpoint: a1b2c3d checkpoint: 2025-02-08 12:30:00
```

```
Nothing to checkpoint (working tree clean).
```

**Worktree create:**

```
Creating worktree:
  Branch: feature/new-thing
  Path:   /Users/saumil/Code/noslop-worktrees/feature/new-thing

cd /Users/saumil/Code/noslop-worktrees/feature/new-thing
```

**Worktree list:**

```
/Users/saumil/Code/noslop                                  a1b2c3d [main] (main)
/Users/saumil/Code/noslop-worktrees/feature/new-thing      b2c3d4e [feature/new-thing]
```

**Worktree remove:**

```
  Removed: /Users/saumil/Code/noslop-worktrees/feature/new-thing
  Deleted merged branch: feature/new-thing
```

**Worktree clean:**

```
  Cleaned 3 worktree(s)
```

### JSON Mode (`--json`)

**Checkpoint:**

```json
{
  "success": true,
  "sha": "a1b2c3d",
  "message": "checkpoint: 2025-02-08 12:30:00"
}
```

**Worktree list:**

```json
[
  {
    "path": "/Users/saumil/Code/noslop",
    "branch": "main",
    "head": "a1b2c3d",
    "is_main": true
  },
  {
    "path": "/Users/saumil/Code/noslop-worktrees/feature/new-thing",
    "branch": "feature/new-thing",
    "head": "b2c3d4e",
    "is_main": false
  }
]
```

## Test Strategy

### Unit Tests (`tests/unit/`)

**Checkpoint adapter tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit so HEAD exists
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        (dir, repo)
    }

    #[test]
    fn is_clean_on_fresh_repo() {
        let (_dir, repo) = setup_repo();
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn is_clean_with_untracked_file() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("new.txt"), "content").unwrap();
        assert!(!is_clean(&repo).unwrap());
    }

    #[test]
    fn checkpoint_stages_and_commits() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = checkpoint(&repo, "test checkpoint").unwrap();
        assert!(result.is_some());
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn checkpoint_returns_none_on_clean_tree() {
        let (_dir, repo) = setup_repo();
        let result = checkpoint(&repo, "nothing here").unwrap();
        assert!(result.is_none());
    }
}
```

**Worktree adapter tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo_with_commit() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create file + initial commit
        std::fs::write(dir.path().join("README.md"), "# test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        (dir, repo)
    }

    #[test]
    fn worktree_base_dir_appends_suffix() {
        let base = worktree_base_dir(Path::new("/code/noslop")).unwrap();
        assert_eq!(base, PathBuf::from("/code/noslop-worktrees"));
    }

    #[test]
    fn list_worktrees_returns_main() {
        let (_dir, repo) = setup_repo_with_commit();
        let worktrees = list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn create_and_list_worktree() {
        let (dir, repo) = setup_repo_with_commit();
        let wt_path = dir.path().parent().unwrap().join("test-worktrees/feature-a");

        create_worktree(&repo, &wt_path, "feature-a", "HEAD").unwrap();

        let worktrees = list_worktrees(&repo).unwrap();
        assert_eq!(worktrees.len(), 2);

        // Cleanup
        std::fs::remove_dir_all(&wt_path).ok();
    }

    #[test]
    fn is_branch_merged_detects_ancestor() {
        let (_dir, repo) = setup_repo_with_commit();
        let head = repo.head().unwrap().peel_to_commit().unwrap();

        // Create branch at HEAD (trivially merged)
        repo.branch("test-branch", &head, false).unwrap();

        assert!(is_branch_merged(&repo, "test-branch").unwrap());
    }
}
```

### Integration Tests (`tests/integration/`)

```rust
#[test]
fn cli_checkpoint_with_changes() {
    let repo = TempGitRepo::new();
    repo.write_file("test.txt", "content");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "test message"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Checkpoint:"));
}

#[test]
fn cli_checkpoint_clean_tree() {
    let repo = TempGitRepo::new();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Nothing to checkpoint"));
}

#[test]
fn cli_checkpoint_json_output() {
    let repo = TempGitRepo::new();
    repo.write_file("test.txt", "content");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "--json", "test"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"success\": true"));
}

#[test]
fn cli_worktree_create_list_remove() {
    let repo = TempGitRepo::new();
    repo.write_file("README.md", "# test");
    repo.commit("initial");

    // Create
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "create", "feature-test"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Branch: feature-test"));

    // List
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("feature-test"));

    // Remove
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "remove", "feature-test"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Removed:"));
}

#[test]
fn cli_worktree_clean() {
    let repo = TempGitRepo::new();
    repo.write_file("README.md", "# test");
    repo.commit("initial");

    // Create two worktrees
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "create", "branch-a"])
        .assert()
        .success();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "create", "branch-b"])
        .assert()
        .success();

    // Clean all
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["worktree", "clean"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Cleaned 2 worktree(s)"));
}
```

## Dependencies

No new crate dependencies are required. All functionality is covered by existing dependencies:

| Crate        | Already in Cargo.toml | Used For                                                      |
| ------------ | --------------------- | ------------------------------------------------------------- |
| `git2`       | Yes (0.20.2)          | All git operations (staging, committing, worktrees, branches) |
| `chrono`     | Yes (0.4.42)          | Default checkpoint timestamp                                  |
| `clap`       | Yes (4.5.51)          | CLI parsing for new subcommands                               |
| `serde_json` | Yes (1.0.145)         | JSON output mode                                              |
| `anyhow`     | Yes (1.0.100)         | Error handling                                                |

## Migration Checklist

This section maps every behavior in the bash scripts to its Rust equivalent, confirming nothing is lost.

### `checkpoint` (56 lines)

| Line(s) | Bash Behavior                                     | Rust Equivalent                    | Status    |
| ------- | ------------------------------------------------- | ---------------------------------- | --------- |
| 17      | Default message with timestamp                    | `chrono::Local::now().format(...)` | Covered   |
| 19      | `detect_repos`                                    | Dropped (single-repo only)         | By design |
| 22-31   | Single-repo: is_clean, add -A, commit --no-verify | `checkpoint::checkpoint()`         | Covered   |
| 29      | Print `git log --oneline -1`                      | Print short SHA + message          | Covered   |
| 34-48   | Multi-repo loop                                   | Dropped                            | By design |
| 50-55   | Summary output                                    | Simplified (single-repo)           | Covered   |

### `worktree` (223 lines)

| Line(s) | Bash Behavior                      | Rust Equivalent                | Status                |
| ------- | ---------------------------------- | ------------------------------ | --------------------- |
| 25-31   | `--repo` flag parsing              | Dropped (single-repo)          | By design             |
| 35      | `detect_repos`                     | Dropped                        | By design             |
| 38-49   | `--repo` filter                    | Dropped                        | By design             |
| 53-56   | `_wt_base`                         | `worktree_base_dir()`          | Covered               |
| 58-73   | `_create_one`                      | `create_worktree()`            | Covered               |
| 75-93   | `_remove_one`                      | `remove_worktree()`            | Covered               |
| 95-119  | `_clean_one`                       | `clean_worktrees()`            | Covered               |
| 123-148 | `create_worktree` (single + multi) | `worktree::create` command     | Covered (single only) |
| 150-160 | `list_worktrees`                   | `worktree::list` command       | Covered               |
| 162-190 | `remove_worktree`                  | `worktree::remove` command     | Covered (single only) |
| 192-202 | `clean_worktrees`                  | `worktree::clean` command      | Covered (single only) |
| 204-222 | Case dispatch + usage              | `WorktreeAction` enum via clap | Covered               |

### `_lib.sh` (77 lines)

| Function         | Rust Equivalent                        | Status       |
| ---------------- | -------------------------------------- | ------------ |
| `detect_repos()` | Dropped (single-repo always)           | By design    |
| `is_clean()`     | `checkpoint::is_clean()`               | Covered      |
| `count_tasks()`  | Separate design (ralph/iteration loop) | Out of scope |
| `git_activity()` | Separate design (ralph/iteration loop) | Out of scope |

## Summary

This design ports checkpoint (56 lines bash) and worktree (223 lines bash) to Rust, fitting cleanly into noslop's hexagonal architecture:

- **CLI layer**: Two new `Command` variants with clap derive, thin dispatch to adapters
- **Port layer**: `VersionControl` trait extended with 8 new methods
- **Adapter layer**: Two new modules under `adapters::git/` using `git2` directly
- **Model layer**: One new `WorktreeInfo` type

Multi-repo mode is deliberately dropped. No new dependencies are required. The `git2` crate handles all operations that the bash scripts achieved via `git` subprocess calls, with the added benefit of no hook execution during checkpoint commits and no output parsing for worktree metadata.
