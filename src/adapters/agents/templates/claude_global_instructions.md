# Global Instructions

## Available Tools

The following custom scripts are available in PATH and auto-allowed in permissions. Use them proactively when appropriate.

### `checkpoint`

Safety commit of all current changes. Use before risky operations, major refactors, or between significant steps.

```
checkpoint                    # Auto-timestamp message
checkpoint "before refactor"  # Custom message
```

### `worktree`

Git worktree management for isolated work.

```
worktree create <branch> [base]  # Create isolated worktree
worktree list                    # List active worktrees
worktree remove <branch>         # Remove worktree
worktree clean                   # Remove all worktrees
```

## Conventions

- Use git flow: work on feature branches, never commit directly to main/master/development/production
- Run `checkpoint` before destructive or risky operations
- Prefer `worktree create` over branching in-place for large parallel tasks
- Never use "enhanced", "extended", "improved", or similar generic prefixes in names
- Type hints required for all Python functions
- TypeScript strict mode for all frontend code
