# Story 02: Branch-Scoped Tasks

**Status: COMPLETE**

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

### US-02.2: Add tasks to current branch
**As a** developer
**I want to** add tasks to my current branch
**So that** I know what work remains

```bash
$ noslop task add "Implement OAuth routes"
Created task: TSK-1

$ noslop task add "Add integration tests" --blocked-by TSK-1
Created task: TSK-2
```

### US-02.3: View tasks for current branch
**As a** developer
**I want to** see tasks for my current branch
**So that** I know what to work on

```bash
$ noslop task list
Branch: feature/oauth

Tasks (2):

  ○ [TSK-1] Implement OAuth routes (p1) [ready]
  ○ [TSK-2] Add integration tests (p1)
      blocked by: TSK-1

$ noslop task list --ready
Branch: feature/oauth

Tasks (1):

  ○ [TSK-1] Implement OAuth routes (p1) [ready]
```

### US-02.4: Switch branches, switch task context
**As a** developer
**I want** task context to switch when I switch branches
**So that** I see relevant tasks for my current work

```bash
$ git checkout feature/oauth
$ noslop task list
Branch: feature/oauth
...

$ git checkout feature/rate-limit
$ noslop task list
Branch: feature/rate-limit
...
```

### US-02.5: Tasks persist locally across sessions
**As a** developer
**I want** my branch's tasks to persist
**So that** I can pick up where I left off tomorrow

```bash
# Monday
$ git checkout feature/oauth
$ noslop task add "Implement OAuth routes"
$ noslop task start TSK-1
# Go home

# Tuesday
$ git checkout feature/oauth
$ noslop task list
Branch: feature/oauth

Tasks (1):

  ◐ [TSK-1] Implement OAuth routes (p1)
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

[branch]
name = "feature/oauth"
created_at = "2025-01-06T10:00:00Z"
parent = "main"

[[task]]
id = 1
title = "Implement OAuth routes"
status = "in_progress"
priority = "p1"
created_at = "2025-01-06T10:00:00Z"

[[task]]
id = 2
title = "Add integration tests"
status = "pending"
priority = "p1"
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
- Replace `/` and other special chars with `-`
- Lowercase

### CLI Commands

```bash
# Task management (scoped to current branch)
noslop task init              # Create task file for current branch
noslop task add "title"       # Add task
noslop task list [--ready]    # List tasks
noslop task start <id>        # Mark in progress
noslop task done <id>         # Mark complete
noslop task remove <id>       # Remove task
```

## Acceptance Criteria

- [x] `.noslop/tasks/` directory created on `noslop init`
- [x] `.noslop/.gitignore` contains `tasks/`
- [x] `noslop task add` creates task in branch-specific file
- [x] `noslop task list` shows branch name and current branch's tasks
- [x] Branch switch shows different tasks
- [x] `--ready` flag filters to unblocked tasks
- [x] `blocked_by` relationships work
- [x] Task files persist across sessions
- [x] Task files don't appear in `git status`
- [x] Integration tests for all functionality

## What's NOT Implemented (Future Work)

- `noslop branch status` command
- `noslop branch switch` command (for post-checkout hook)
- Automatic task file creation on branch creation
- `noslop migrate` for existing tasks in `.noslop.toml`
