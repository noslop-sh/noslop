# Story 04: Sub-Branch Hierarchy

## Summary

Support hierarchical branch naming (e.g., `feature/oauth/routes`) where sub-branches represent subtasks of parent branches. Enable agents to create sub-branches for focused work, then merge back.

## Motivation

**Current problem:**
- Large features require breaking down into smaller pieces
- No structured way to represent subtasks
- Agents working on big features lose focus

**Solution:**
- Branch naming convention creates hierarchy: `parent/child`
- Sub-branches are subtasks of parent branch
- Agent can create sub-branch, work focused, merge back
- Parent branch tracks overall progress

## User Stories

### US-04.1: View branch hierarchy
**As a** developer
**I want** to see branches as a tree
**So that** I understand the work structure

```bash
$ noslop branch tree

main
└── feature/oauth
    ├── feature/oauth/routes (in progress)
    ├── feature/oauth/refresh-tokens (pending)
    └── feature/oauth/tests (blocked)
```

### US-04.2: Create sub-branch
**As a** developer
**I want** to create a sub-branch for focused work
**So that** I can work on one piece at a time

```bash
$ git checkout feature/oauth
$ noslop branch create routes

Created: feature/oauth/routes
Created: .noslop/tasks/feature-oauth-routes.toml
Switched to branch 'feature/oauth/routes'
```

### US-04.3: Sub-branch inherits parent context
**As a** developer
**I want** sub-branches to see parent's notes
**So that** context flows down

```bash
$ git checkout feature/oauth/routes
$ noslop branch context

Branch: feature/oauth/routes
Parent: feature/oauth

Parent notes:
  See RFC: docs/rfcs/oauth.md
  Security review required

This branch:
  Implementing OAuth routes and handlers
```

### US-04.4: Get next subtask as sub-branch
**As an** agent
**I want** to get the next unblocked subtask
**So that** I can work on it in isolation

```bash
$ git checkout feature/oauth
$ noslop task next

Next: routes - Implement OAuth routes

$ noslop branch create routes
Switched to branch 'feature/oauth/routes'
Started: Task 'routes'
```

### US-04.5: Merge sub-branch back
**As a** developer
**I want** to merge completed sub-branch into parent
**So that** progress is integrated

```bash
$ git checkout feature/oauth/routes
$ git commit -m "Complete OAuth routes"

Task 'routes' - done/continue? d
✓ Completed: routes

$ noslop branch done

Merging feature/oauth/routes → feature/oauth...
✓ Merged
✓ Task 'routes' marked complete in parent

Switched to branch 'feature/oauth'
Next ready: refresh-tokens
```

### US-04.6: Clean up merged sub-branches
**As a** developer
**I want** merged sub-branches cleaned up
**So that** branch list stays manageable

```bash
$ noslop branch gc

Cleaning up merged branches...
  Deleted: feature/oauth/routes (merged)
  Deleted: .noslop/tasks/feature-oauth-routes.toml

  Kept: feature/oauth/tests (not merged)
```

## Technical Design

### Branch Hierarchy Detection

```
feature/oauth           → parent (depth 1)
feature/oauth/routes    → child of feature/oauth (depth 2)
feature/oauth/routes/v2 → child of feature/oauth/routes (depth 3)
```

Logic:
```rust
fn get_parent_branch(branch: &str) -> Option<String> {
    let parts: Vec<&str> = branch.rsplitn(2, '/').collect();
    if parts.len() == 2 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

// feature/oauth/routes → Some("feature/oauth")
// feature/oauth → None (or "main"?)
```

### Task File Relationships

Parent task file can reference subtasks as sub-branches:

```toml
# .noslop/tasks/feature-oauth.toml

[[task]]
id = 1
title = "Implement OAuth routes"
branch = "feature/oauth/routes"  # Links to sub-branch
status = "done"

[[task]]
id = 2
title = "Add refresh token logic"
branch = "feature/oauth/refresh-tokens"
status = "pending"
blocked_by = [1]

[[task]]
id = 3
title = "Add integration tests"
branch = "feature/oauth/tests"
status = "pending"
blocked_by = [1, 2]
```

### CLI Commands

```bash
# Branch hierarchy
noslop branch tree            # Show branch tree
noslop branch create <name>   # Create sub-branch of current
noslop branch done            # Merge current into parent
noslop branch gc              # Clean up merged branches

# Task with branch creation
noslop task next              # Get next unblocked task
noslop task start <id> --branch  # Start task as sub-branch
```

### Merge Flow

```
┌─────────────────────────────────────────────────────────┐
│                   noslop branch done                     │
├─────────────────────────────────────────────────────────┤
│  1. Ensure current branch has parent                    │
│  2. Ensure all checks pass                              │
│  3. Prompt: merge to parent? [y/n]                      │
│  4. git checkout <parent>                               │
│  5. git merge <current>                                 │
│  6. Update parent's task file (mark subtask done)       │
│  7. Show next ready task                                │
│  8. Optionally delete sub-branch                        │
└─────────────────────────────────────────────────────────┘
```

### Agent Workflow

```bash
# Agent on feature/oauth
$ noslop task next
routes - Implement OAuth routes

# Agent creates sub-branch
$ noslop task start 1 --branch
Created: feature/oauth/routes
Switched to feature/oauth/routes

# Agent works...
$ git commit -m "Add OAuth handlers"
Task 'routes' - done/continue? c

$ git commit -m "Add OAuth middleware"
Task 'routes' - done/continue? d
✓ Completed

# Agent merges back
$ noslop branch done
Merged feature/oauth/routes → feature/oauth
Next: refresh-tokens

# Agent continues with next subtask
$ noslop task start 2 --branch
Created: feature/oauth/refresh-tokens
```

## Acceptance Criteria

- [ ] `noslop branch tree` shows hierarchy
- [ ] `noslop branch create <name>` creates sub-branch
- [ ] Sub-branch task file created automatically
- [ ] Parent context visible from sub-branch
- [ ] `noslop branch done` merges to parent
- [ ] Parent task marked complete after merge
- [ ] `noslop branch gc` cleans merged branches
- [ ] `noslop task start --branch` creates and switches

## Edge Cases

1. **Deep nesting**: `a/b/c/d` - support arbitrary depth
2. **Orphan branches**: Sub-branch exists but parent deleted - treat as root
3. **Merge conflicts**: `noslop branch done` fails gracefully, user resolves
4. **Concurrent sub-branches**: Multiple sub-branches can exist, each independent

## Open Questions

1. Max depth limit? (Probably not needed)
2. Should `noslop branch create` auto-create parent task?
3. Should parent branch's checks apply to sub-branches?
4. What happens if you `noslop branch done` with uncommitted changes?
