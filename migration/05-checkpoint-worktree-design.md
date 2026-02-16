# Checkpoint & VersionControl Extensions Design

Design specification for the `noslop checkpoint` convenience command and `VersionControl` trait extensions that serve the review pipeline.

## Overview

noslop is a local code review tool that uses a pipeline of `ReviewAnalyzer` implementations to review agent-generated branches before push. The `VersionControl` trait needs extensions in two areas:

1. **Checkpoint**: A standalone convenience command for developers who want to save their work before making fixes to review findings. This is a simple utility, not a core loop mechanism.

2. **Diff and history methods**: The review pipeline needs to read diffs, commit history, and file contents at specific revisions. These are the primary trait extensions and the most important part of this design.

**Worktree support is deferred.** The review tool does not need isolated worktrees for parallel coding. Worktrees could be useful in the future for isolated review environments (e.g., running analyzers against a branch without checking it out), but that is a future consideration.

### Scope

| Component          | Purpose                                           | Rust Target                      |
| ------------------ | ------------------------------------------------- | -------------------------------- |
| Checkpoint command | Convenience: save work before fixing findings     | `src/cli/commands/checkpoint.rs` |
| Checkpoint adapter | Stage all + commit with message                   | `src/adapters/git/checkpoint.rs` |
| Review VCS methods | Diff, commit history, file-at-revision for review | Extend `src/adapters/git/mod.rs` |

### What Changes

- One new CLI subcommand added to `Command` enum
- One new adapter module under `src/adapters/git/`
- `VersionControl` trait extended with checkpoint + review pipeline methods
- No changes to existing core domain logic

### What Does NOT Ship

- Worktree management (create/remove/clean) -- deferred
- `WorktreeInfo` model type -- deferred
- Multi-repo detection -- dropped permanently
- Iteration loop integration -- not applicable (noslop is a review tool)

## File Layout

```
src/
├── cli/
│   ├── app.rs                    # Add Checkpoint to Command enum
│   └── commands/
│       ├── mod.rs                # Add checkpoint re-export
│       └── checkpoint.rs         # NEW: checkpoint command handler
├── core/
│   ├── ports/
│   │   └── vcs.rs                # Extend VersionControl trait
│   └── models/
│       └── diff.rs               # NEW: FileDiff, CommitInfo types
└── adapters/
    └── git/
        ├── mod.rs                # Add checkpoint module
        └── checkpoint.rs         # NEW: git2-based checkpoint operations
```

## CLI Interface

### `noslop checkpoint`

```
noslop checkpoint                         # Message: "checkpoint: 2025-02-08 12:30:00"
noslop checkpoint "before fixing findings" # Custom message
noslop checkpoint --json                  # JSON output
```

This is a convenience command. A developer runs `noslop review`, sees findings, wants to checkpoint their current state before making fixes. That is the entire use case.

### Worktree (Deferred)

The `noslop worktree` subcommand is deferred. The review tool does not need isolated worktrees. If worktree support is added in the future, it would likely be limited to `noslop worktree list` for awareness of existing worktrees, not the full create/remove/clean suite.

**Future consideration**: Isolated review environments. An analyzer could check out a branch in a temporary worktree to run static analysis without affecting the developer's working tree. This is speculative and not designed here.

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
}
```

### Commands Module Extension

```rust
// In commands/mod.rs:
mod checkpoint;

pub use checkpoint::checkpoint;
```

## VersionControl Trait Extension

The `VersionControl` trait in `src/core/ports/vcs.rs` gains two categories of methods:

1. **Checkpoint methods**: `is_clean()` and `commit_all()` for the checkpoint command
2. **Review pipeline methods**: `diff_between()`, `commits_between()`, `file_at_revision()`, `recent_commits()`, and `diff_stat_since()` for analyzers

The review pipeline methods are the primary reason for this trait extension. Analyzers need diff context and commit history to produce meaningful findings.

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

    // --- checkpoint methods ---

    /// Check if the working tree is clean (no staged, unstaged, or untracked changes)
    fn is_clean(&self) -> anyhow::Result<bool>;

    /// Stage all changes and commit with the given message, bypassing hooks
    ///
    /// Returns the short SHA of the created commit, or None if the tree was clean.
    fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>>;

    // --- review pipeline methods ---

    /// Get the diff between two refs (base and head)
    ///
    /// This is the primary input to the review pipeline. Analyzers receive
    /// the list of file diffs to inspect for findings.
    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>>;

    /// List commits between two refs
    ///
    /// Used by analyzers that inspect commit messages and patterns
    /// (e.g., checking for conventional commits, squash candidates).
    fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>>;

    /// Read file content at a specific revision
    ///
    /// Used by analyzers that need full file context beyond the diff hunks
    /// (e.g., checking imports, verifying function signatures).
    fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String>;

    /// Get recent commits from HEAD (for context)
    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;

    /// Get a summary of changes since a given ref
    ///
    /// Returns (files_changed, insertions, deletions) for display purposes.
    fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat>;
}
```

### New Model Types

```rust
// src/core/models/diff.rs

use std::path::PathBuf;

/// A diff for a single file between two revisions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// Path of the file (new path if renamed)
    pub path: PathBuf,
    /// Old path if the file was renamed
    pub old_path: Option<PathBuf>,
    /// The kind of change
    pub status: DiffStatus,
    /// Individual hunks in the diff
    pub hunks: Vec<DiffHunk>,
}

/// What happened to a file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// A single hunk within a file diff
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// Starting line in old file
    pub old_start: u32,
    /// Number of lines in old file
    pub old_lines: u32,
    /// Starting line in new file
    pub new_start: u32,
    /// Number of lines in new file
    pub new_lines: u32,
    /// The actual diff content (unified format lines)
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff hunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

/// Information about a single commit
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Short SHA (7 chars)
    pub short_sha: String,
    /// Full SHA
    pub sha: String,
    /// Commit message (first line)
    pub summary: String,
    /// Full commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Timestamp (seconds since epoch)
    pub timestamp: i64,
}

/// Summary statistics for a diff
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}
```

Added to `src/core/models/mod.rs`:

```rust
mod diff;
pub use diff::{FileDiff, DiffStatus, DiffHunk, DiffLine, DiffLineKind, CommitInfo, DiffStat};
```

## Adapter Implementations

### `src/adapters/git/checkpoint.rs`

Implements the checkpoint operation using `git2`. Simple: check if clean, stage all, commit.

```rust
//! Checkpoint operations using git2
//!
//! Stages all changes (tracked + untracked) and commits with --no-verify.

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
pub fn commit_all(repo: &Repository, message: &str) -> anyhow::Result<Option<String>> {
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

### Review Pipeline Methods (in `GitVersionControl`)

These methods are implemented directly on `GitVersionControl` in `src/adapters/git/mod.rs` since they are read-only queries against the repository.

```rust
impl VersionControl for GitVersionControl {
    // ... existing methods ...

    fn is_clean(&self) -> anyhow::Result<bool> {
        let repo = git2::Repository::discover(&self.workdir)?;
        checkpoint::is_clean(&repo)
    }

    fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>> {
        let repo = git2::Repository::discover(&self.workdir)?;
        checkpoint::commit_all(&repo, message)
    }

    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>> {
        let repo = git2::Repository::discover(&self.workdir)?;

        let base_tree = repo.revparse_single(base)?.peel_to_tree()?;
        let head_tree = repo.revparse_single(head)?.peel_to_tree()?;

        let diff = repo.diff_tree_to_tree(
            Some(&base_tree),
            Some(&head_tree),
            None,
        )?;

        let mut file_diffs = Vec::new();
        // ... convert git2::Diff to Vec<FileDiff> ...
        // (Implementation parses diff deltas, hunks, and lines
        //  into the FileDiff/DiffHunk/DiffLine model types)

        Ok(file_diffs)
    }

    fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>> {
        let repo = git2::Repository::discover(&self.workdir)?;

        let base_oid = repo.revparse_single(base)?.id();
        let head_oid = repo.revparse_single(head)?.id();

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;
        revwalk.hide(base_oid)?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            commits.push(CommitInfo {
                short_sha: oid.to_string()[..7].to_string(),
                sha: oid.to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String> {
        let repo = git2::Repository::discover(&self.workdir)?;

        let obj = repo.revparse_single(rev)?;
        let commit = obj.peel_to_commit()?;
        let tree = commit.tree()?;

        let entry = tree.get_path(path)?;
        let blob = repo.find_blob(entry.id())?;

        let content = std::str::from_utf8(blob.content())
            .map_err(|_| anyhow::anyhow!("File is not valid UTF-8: {}", path.display()))?;

        Ok(content.to_string())
    }

    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>> {
        let repo = git2::Repository::discover(&self.workdir)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;

        let mut commits = Vec::new();
        for oid in revwalk.take(count) {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            commits.push(CommitInfo {
                short_sha: oid.to_string()[..7].to_string(),
                sha: oid.to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat> {
        let repo = git2::Repository::discover(&self.workdir)?;

        let since_tree = repo.revparse_single(since)?.peel_to_tree()?;
        let head_tree = repo.head()?.peel_to_tree()?;

        let diff = repo.diff_tree_to_tree(
            Some(&since_tree),
            Some(&head_tree),
            None,
        )?;

        let stats = diff.stats()?;
        Ok(DiffStat {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }
}
```

## `GitVersionControl` Module Registration

```rust
// src/adapters/git/mod.rs

pub mod hooks;
pub mod staging;
pub mod checkpoint;   // NEW

// ... existing code ...

impl VersionControl for GitVersionControl {
    // ... existing methods + new methods above ...
}
```

## Command Implementation

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

    match vcs.commit_all(&msg)? {
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

## How the Review Pipeline Uses These Methods

The review pipeline is the primary consumer of the `VersionControl` trait extensions. Here is how each method maps to the review workflow:

```
noslop review [--base main] [--head HEAD]
        │
        ▼
┌─────────────────────────────────────────────────┐
│  Review Pipeline                                 │
│                                                  │
│  1. vcs.diff_between(base, head)                 │
│     → Vec<FileDiff> — the core review input       │
│                                                  │
│  2. vcs.commits_between(base, head)              │
│     → Vec<CommitInfo> — context for analyzers     │
│                                                  │
│  3. For each analyzer in pipeline:               │
│     analyzer.analyze(&file_diffs, &commits)      │
│                                                  │
│  4. If analyzer needs full file context:         │
│     vcs.file_at_revision(path, head)             │
│     → String — complete file at head revision     │
│                                                  │
│  5. Collect and deduplicate findings             │
│  6. Display or output as JSON                    │
└─────────────────────────────────────────────────┘
```

### Example: Static Analyzer Using Diffs

```rust
impl ReviewAnalyzer for ConventionsAnalyzer {
    fn analyze(&self, context: &ReviewContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for file_diff in &context.file_diffs {
            // Check added lines against convention rules
            for hunk in &file_diff.hunks {
                for line in &hunk.lines {
                    if line.kind == DiffLineKind::Addition {
                        // Run convention checks on new code only
                        findings.extend(self.check_line(&file_diff.path, line));
                    }
                }
            }
        }

        Ok(findings)
    }
}
```

### Example: LLM Analyzer Using File Context

```rust
impl ReviewAnalyzer for AgentSecurityAnalyzer {
    fn analyze(&self, context: &ReviewContext) -> anyhow::Result<Vec<Finding>> {
        // Get full file contents for files with security-relevant changes
        let security_files = context.file_diffs.iter()
            .filter(|d| self.is_security_relevant(&d.path));

        for file_diff in security_files {
            let content = context.vcs.file_at_revision(
                &file_diff.path,
                &context.head,
            )?;

            // Send to LLM agent for security review
            let findings = self.agent.review_file(
                &file_diff,
                &content,
                &context.commits,
            )?;

            // ... collect findings ...
        }

        Ok(findings)
    }
}
```

## Architecture Integration

### Hexagonal Boundaries

```
┌───────────────────────────────────────────────────────────────┐
│                    CLI Layer (Thin Dispatch)                    │
│                                                                │
│  checkpoint.rs       review.rs                                 │
│  - Format output     - Orchestrate review pipeline             │
│  - Handle modes      - Display findings                        │
└───────────┬──────────────────────┬────────────────────────────┘
            │                      │
┌───────────▼──────────────────────▼────────────────────────────┐
│                    Port Layer (Traits)                          │
│                                                                │
│  VersionControl                                                │
│  + is_clean() -> bool                                          │
│  + commit_all(msg) -> Option<sha>                              │
│  + diff_between(base, head) -> Vec<FileDiff>                   │
│  + commits_between(base, head) -> Vec<CommitInfo>              │
│  + file_at_revision(path, rev) -> String                       │
│  + recent_commits(count) -> Vec<CommitInfo>                    │
│  + diff_stat_since(since) -> DiffStat                          │
└───────────┬──────────────────────┬────────────────────────────┘
            │                      │
┌───────────▼──────────────────────▼────────────────────────────┐
│                    Adapter Layer (I/O)                          │
│                                                                │
│  GitVersionControl                                             │
│  ├── checkpoint.rs   (git2::Repository)                        │
│  │   - is_clean()    → repo.statuses()                         │
│  │   - commit_all()  → index.add_all + repo.commit             │
│  └── (inline)        (git2::Repository)                        │
│      - diff_between  → repo.diff_tree_to_tree                  │
│      - commits_between → repo.revwalk                          │
│      - file_at_revision → tree.get_path + find_blob            │
│      - recent_commits → repo.revwalk (from HEAD)               │
│      - diff_stat_since → diff.stats()                          │
└───────────────────────────────────────────────────────────────┘
```

### Why VersionControl Trait (not a new port)

All these methods interact with the same git repository as `staged_files`, `repo_root`, and `current_branch`. Creating a separate `ReviewVcsPort` would fragment the abstraction:

1. **Same boundary**: All operations are reads/writes against a single git repository.
2. **Same adapter**: `GitVersionControl` already holds the `workdir` path needed by all operations.
3. **Review pipeline composes them**: A review run calls `diff_between`, `commits_between`, and `file_at_revision` on the same VCS instance.
4. **Noslop convention**: Existing code places all git operations behind `VersionControl`.

### Why `git2` Instead of `std::process::Command`

1. **`git2` is already a dependency** (v0.20.2 with vendored OpenSSL).
2. **No subprocess overhead**: Diff computation and file reads happen in-process.
3. **Structured data**: `git2` returns typed diff objects, avoiding `git diff` output parsing.
4. **Atomic operations**: `git2`'s index API stages files in a single operation for checkpoint.
5. **No hook execution**: `git2` commits do not invoke git hooks, which is the desired checkpoint behavior.

## Output Format

### Human Mode (default)

**Checkpoint:**

```
Checkpoint: a1b2c3d checkpoint: 2025-02-08 12:30:00
```

```
Nothing to checkpoint (working tree clean).
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

## Test Strategy

### Unit Tests

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
    fn commit_all_stages_and_commits() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = commit_all(&repo, "test checkpoint").unwrap();
        assert!(result.is_some());
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_returns_none_on_clean_tree() {
        let (_dir, repo) = setup_repo();
        let result = commit_all(&repo, "nothing here").unwrap();
        assert!(result.is_none());
    }
}
```

**Review pipeline method tests:**

```rust
#[cfg(test)]
mod review_vcs_tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo_with_branch() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit on main
        std::fs::write(dir.path().join("README.md"), "# test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        let commit = repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        // Create a feature branch with a change
        let head = repo.find_commit(commit).unwrap();
        repo.branch("feature", &head, false).unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        std::fs::write(dir.path().join("new_file.rs"), "fn main() {}").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new_file.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add new file", &tree, &[&head]).unwrap();

        (dir, repo)
    }

    #[test]
    fn diff_between_shows_added_file() {
        let (_dir, repo) = setup_repo_with_branch();
        // Test that diff between main and feature shows new_file.rs as added
        // (Implementation would use the GitVersionControl wrapper)
    }

    #[test]
    fn commits_between_lists_feature_commits() {
        let (_dir, repo) = setup_repo_with_branch();
        // Test that commits_between(main, feature) returns the feature commit
    }

    #[test]
    fn file_at_revision_reads_content() {
        let (_dir, repo) = setup_repo_with_branch();
        // Test that file_at_revision("new_file.rs", "feature") returns "fn main() {}"
    }
}
```

### Integration Tests

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
```

## Dependencies

No new crate dependencies are required:

| Crate        | Already in Cargo.toml | Used For                                   |
| ------------ | --------------------- | ------------------------------------------ |
| `git2`       | Yes (0.20.2)          | All git operations (diff, log, checkpoint) |
| `chrono`     | Yes (0.4.42)          | Default checkpoint timestamp               |
| `clap`       | Yes (4.5.51)          | CLI parsing for checkpoint subcommand      |
| `serde_json` | Yes (1.0.145)         | JSON output mode                           |
| `anyhow`     | Yes (1.0.100)         | Error handling                             |

## Summary

This design serves two purposes:

1. **Checkpoint**: A simple convenience command (`noslop checkpoint [msg]`) that stages all changes and commits. Useful for developers who want to save state before fixing review findings. One new CLI subcommand, one new adapter module, two trait methods (`is_clean`, `commit_all`).

2. **Review pipeline VCS methods**: The primary trait extensions. Five new methods on `VersionControl` that give analyzers access to diffs (`diff_between`), commit history (`commits_between`, `recent_commits`), file contents (`file_at_revision`), and change statistics (`diff_stat_since`). These are the foundation that every `ReviewAnalyzer` builds on.

Worktree management is deferred as a future consideration. The review tool does not currently need isolated worktrees.
