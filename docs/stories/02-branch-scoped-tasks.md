# Story 02: Branch-Scoped Tasks

## Summary

Tasks are scoped to git branches. Each branch has its own task file stored in `.noslop/tasks/{branch}.toml`. Task files are gitignored (local scratch), while task completions are recorded in commit trailers (permanent record).

## Motivation

**Current problem:**
- Tasks in `.noslop.toml` are global, creating noise in git history
- Task state changes (started, done) pollute commits
- No natural mapping between tasks and the work being done

**Solution:**
- Branch = unit of work (developers already think this way)
- Task file = notes/checklist for that branch's work
- Gitignored = no merge conflicts, no commit noise
- Commit trailers = permanent, auditable record

## User Stories

### US-02.1: Create task file for new branch
**As a** developer
**I want** a task file created when I start a new branch
**So that** I can track what needs to be done on this branch

```bash
$ git checkout -b feature/oauth
$ noslop task init
Created: .noslop/tasks/feature-oauth.toml
```

Or automatically via post-checkout hook.

### US-02.2: Add tasks to current branch
**As a** developer
**I want to** add tasks to my current branch
**So that** I know what work remains

```bash
$ noslop task add "Implement OAuth routes"
Added: [1] Implement OAuth routes

$ noslop task add "Add integration tests" --blocked-by 1
Added: [2] Add integration tests (blocked by: 1)
```

### US-02.3: View tasks for current branch
**As a** developer
**I want to** see tasks for my current branch
**So that** I know what to work on

```bash
$ noslop task list
Branch: feature/oauth

[1] ○ Implement OAuth routes
[2] ⊘ Add integration tests (blocked by: 1)

$ noslop task list --ready
[1] ○ Implement OAuth routes
```

### US-02.4: Switch branches, switch task context
**As a** developer
**I want** task context to switch when I switch branches
**So that** I see relevant tasks for my current work

```bash
$ git checkout feature/oauth
$ noslop task list
Branch: feature/oauth
[1] ○ Implement OAuth routes

$ git checkout feature/rate-limit
$ noslop task list
Branch: feature/rate-limit
[1] ○ Add rate limiting middleware
```

### US-02.5: Tasks persist locally across sessions
**As a** developer
**I want** my branch's tasks to persist
**So that** I can pick up where I left off tomorrow

```bash
# Monday
$ git checkout feature/oauth
$ noslop task add "Implement OAuth routes"
$ noslop task start 1
# Go home

# Tuesday
$ git checkout feature/oauth
$ noslop task list
[1] ◐ Implement OAuth routes (in progress)
```

### US-02.6: Gitignored task files
**As a** developer
**I want** task files to be gitignored
**So that** they don't create merge conflicts or commit noise

```bash
$ cat .noslop/.gitignore
tasks/
```

## Technical Design

### Directory Structure
```
.noslop/
├── .gitignore          # Contains: tasks/
└── tasks/
    ├── main.toml
    ├── feature-oauth.toml
    └── feature-rate-limit.toml
```

### Task File Format
```toml
# .noslop/tasks/feature-oauth.toml

# Branch metadata
[branch]
name = "feature/oauth"
created_at = "2025-01-06T10:00:00Z"
parent = "main"

# Notes for this branch's work
notes = """
Implementing OAuth login flow.
See RFC: docs/rfcs/oauth.md
"""

# Tasks
[[task]]
id = 1
title = "Implement OAuth routes"
status = "in_progress"
created_at = "2025-01-06T10:00:00Z"

[[task]]
id = 2
title = "Add integration tests"
status = "pending"
blocked_by = [1]
created_at = "2025-01-06T10:05:00Z"
```

### Branch Name to Filename Mapping
```
feature/oauth        → feature-oauth.toml
fix/bug-123          → fix-bug-123.toml
feature/oauth/routes → feature-oauth-routes.toml
```

Rules:
- Replace `/` with `-`
- Replace other special chars with `-`
- Lowercase

### Git Hooks

**post-checkout hook:**
```bash
#!/bin/bash
NEW_BRANCH=$(git rev-parse --abbrev-ref HEAD)
noslop branch switch "$NEW_BRANCH"
```

This ensures task file exists and shows summary.

### CLI Commands

```bash
# Task management (scoped to current branch)
noslop task init              # Create task file for current branch
noslop task add "title"       # Add task
noslop task list [--ready]    # List tasks
noslop task start <id>        # Mark in progress
noslop task done <id>         # Mark complete
noslop task remove <id>       # Remove task

# Branch info
noslop branch status          # Show current branch's task summary
noslop branch switch <name>   # Called by post-checkout hook
```

## Acceptance Criteria

- [ ] `.noslop/tasks/` directory created on `noslop init`
- [ ] `.noslop/.gitignore` contains `tasks/`
- [ ] `noslop task add` creates task in branch-specific file
- [ ] `noslop task list` shows only current branch's tasks
- [ ] Branch switch shows different tasks
- [ ] `--ready` flag filters to unblocked tasks
- [ ] `blocked_by` relationships work
- [ ] Task files persist across sessions
- [ ] Task files don't appear in `git status`

## Migration Path

Current tasks in `.noslop.toml`:
1. `noslop migrate` moves `[[task]]` entries to `.noslop/tasks/main.toml`
2. Or leave them (global tasks still work, branch tasks are additive)

## Open Questions

1. Should `noslop task init` be automatic on branch creation?
2. What happens to task file when branch is deleted?
3. Should we support `noslop task move <id> <branch>` for moving tasks between branches?
