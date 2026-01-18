# Noslop Storage Layout

This document describes where noslop stores data on the filesystem.

## Design Principles

Noslop follows the same patterns as git:

| Noslop | Git Equivalent | Purpose |
|--------|----------------|---------|
| `.noslop.toml` | `.gitattributes` / `.gitignore` | Committed project config |
| `.noslop/` | `.git/` | Local state (gitignored) |
| `~/.config/noslop/config.toml` | `~/.gitconfig` | User preferences |

## Per-Project Storage

Located at the repository root (or main worktree for git worktrees):

```
repo/
├── .noslop.toml              # Committed - project config
└── .noslop/                  # Gitignored - local state
    ├── HEAD                  # Current active task ID
    ├── refs/tasks/           # Task storage (one file per task)
    │   ├── TSK-1
    │   ├── TSK-2
    │   └── ...
    ├── staged-verifications.json   # Temp staging before commit
    └── .gitignore            # Excludes refs/, HEAD
```

### `.noslop.toml` (Committed)

Project configuration file. Should be committed to version control.

```toml
[project]
prefix = "NOS"  # Task/check ID prefix

[[check]]
id = "NOS-1"
scope = "src/**/*.rs"
message = "Consider impact on public API"
severity = "block"  # block | warn | info
```

### `.noslop/` (Gitignored)

Local state directory. Contains task state that is NOT committed.

- **`HEAD`** - Plain text file containing the current active task ID
- **`refs/tasks/<ID>`** - JSON files, one per task
- **`staged-verifications.json`** - Temporary staging area for check verifications (cleared after commit)

## Global Storage

User-level configuration at `~/.config/noslop/config.toml`:

```toml
[ui]
theme = "dark"

[workspaces."/path/to/workspace"]
repos = ["/path/to/repo1", "/path/to/repo2"]

[workspaces."/path/to/workspace".branches."/path/to/repo1"]
selected = ["main", "feature-x"]
hidden = ["wip", "archived"]

[workspaces."/path/to/workspace".colors]
"repo1/main" = 0
"repo1/feature-x" = 1

[[workspaces."/path/to/workspace".concepts]]
id = "CON-1"
name = "Authentication"
description = "User auth and session management"
scope = ["src/auth/**"]
created_at = "2026-01-17T12:00:00Z"
```

## Git Integration

Noslop also stores data in git itself:

### Git Hooks (`.git/hooks/`)

Installed by `noslop init`:
- `pre-commit` - Runs check verification
- `commit-msg` - Injects verification trailers
- `post-commit` - Clears staging area

### Commit Trailers

Verifications are stored permanently in commit messages:

```
feat: Add user authentication

Noslop-Verify: NOS-1 | Reviewed API impact | claude
Noslop-Task: TSK-5 | done | Implement auth flow
```

## Worktree Support

When running from a git worktree, all `.noslop/` paths resolve to the **main worktree root**. This ensures all worktrees share the same task state.

## Code Reference

All paths are defined in `src/paths.rs`. This is the single source of truth for filesystem locations.

```rust
use noslop::paths;

// Project paths (worktree-aware)
paths::noslop_toml()         // .noslop.toml
paths::noslop_dir()          // .noslop/
paths::head_file()           // .noslop/HEAD
paths::refs_tasks_dir()      // .noslop/refs/tasks/
paths::task_ref("TSK-1")     // .noslop/refs/tasks/TSK-1
paths::staged_verifications() // .noslop/staged-verifications.json

// Global paths
paths::global_config_dir()   // ~/.config/noslop/
paths::global_config()       // ~/.config/noslop/config.toml
```
