# Story 03: Commit-Time Task Completion

## Summary

At commit time, after checks pass, prompt the user whether the current task is done or continuing. Record task completion in commit trailers, creating a permanent audit trail.

## Motivation

**Current problem:**
- Task completion is a separate step (`noslop task done`)
- Easy to forget to update task status
- No connection between commits and tasks

**Solution:**
- Commit hook asks "done or continue?" after checks pass
- Task completion recorded in commit trailers
- One interaction handles checks + task status
- Git history becomes the task completion record

## User Stories

### US-03.1: Prompt for task status at commit
**As a** developer
**I want** to be asked if my task is done when I commit
**So that** task tracking is part of my natural workflow

```bash
$ git commit -m "Add OAuth routes"

CHECKS (1 file)
  [NOS-1] src/auth/oauth.py
          Security review required? ✓ verified

Task 'Implement OAuth routes' is in progress.
Is this commit: [d]one, [c]ontinue, [s]kip? d

✓ Committed: Add OAuth routes
✓ Completed: Task 1 - Implement OAuth routes
```

### US-03.2: Continue working on task
**As a** developer
**I want** to make mid-task commits
**So that** I can save progress without marking the task done

```bash
$ git commit -m "WIP: OAuth database schema"

CHECKS (0 blocking)

Task 'Implement OAuth routes' is in progress.
Is this commit: [d]one, [c]ontinue, [s]kip? c

✓ Committed: WIP: OAuth database schema
  Task 1 still in progress
```

### US-03.3: Skip task prompt
**As a** developer
**I want** to skip the task prompt for unrelated commits
**So that** quick fixes don't require task management

```bash
$ git commit -m "Fix typo in README"

CHECKS (0 blocking)

Task 'Implement OAuth routes' is in progress.
Is this commit: [d]one, [c]ontinue, [s]kip? s

✓ Committed: Fix typo in README
```

### US-03.4: Task completion in commit trailers
**As a** developer
**I want** task completions recorded in commit history
**So that** there's an audit trail

```bash
$ git log -1 --format="%B"
Add OAuth routes

Noslop-Verify: NOS-1 | Security review | human
Noslop-Task: 1 | done | Implement OAuth routes
Noslop-Branch: feature/oauth
```

### US-03.5: No prompt when no task in progress
**As a** developer
**I want** to skip task prompts when I haven't started a task
**So that** the workflow isn't intrusive

```bash
$ noslop task list
[1] ○ Implement OAuth routes (pending, not started)

$ git commit -m "Update dependencies"

CHECKS (0 blocking)

# No task prompt - nothing in progress

✓ Committed: Update dependencies
```

### US-03.6: View task history from git log
**As a** developer
**I want** to see what tasks were completed
**So that** I can understand project progress

```bash
$ git log --grep="Noslop-Task:" --format="%h %s"
abc123 Add OAuth routes
def456 Add rate limiting
```

```bash
$ noslop history
Recent task completions:
  abc123  feature/oauth       Task 1: Implement OAuth routes
  def456  feature/rate-limit  Task 1: Add rate limiting
```

## Technical Design

### Commit Hook Flow

```
┌─────────────────────────────────────────────────────────┐
│                    pre-commit hook                       │
├─────────────────────────────────────────────────────────┤
│  1. Get staged files                                    │
│  2. Run checks                                          │
│  3. If blocking checks unverified → EXIT 1              │
│  4. Get current branch's in_progress task               │
│  5. If task exists:                                     │
│     - Prompt: done/continue/skip                        │
│     - If done: mark task complete, prepare trailer      │
│  6. EXIT 0                                              │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│               prepare-commit-msg hook                    │
├─────────────────────────────────────────────────────────┤
│  1. Read staged verifications                           │
│  2. Read task completion (if any)                       │
│  3. Append trailers to commit message                   │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   post-commit hook                       │
├─────────────────────────────────────────────────────────┤
│  1. Clear staged verifications                          │
│  2. Update task file (mark done if completed)           │
│  3. Show next ready task (if any)                       │
└─────────────────────────────────────────────────────────┘
```

### Trailer Format

```
Noslop-Verify: <check-id> | <message> | <by>
Noslop-Task: <task-id> | <status> | <title>
Noslop-Branch: <branch-name>
```

Examples:
```
Noslop-Verify: NOS-1 | Ran alembic --autogenerate | human
Noslop-Verify: NOS-2 | Reviewed security implications | claude
Noslop-Task: 1 | done | Implement OAuth routes
Noslop-Task: 2 | partial | Add integration tests
Noslop-Branch: feature/oauth
```

### Task Status Values

| Status | Meaning | Trailer |
|--------|---------|---------|
| `done` | Task complete | `Noslop-Task: 1 \| done \| ...` |
| `partial` | Progress made, not done | `Noslop-Task: 1 \| partial \| ...` |
| (skip) | No trailer added | — |

### CLI Integration

```bash
# CI mode: no interactive prompts
noslop check run --ci

# Non-interactive done
git commit -m "msg" && noslop task done 1

# Or environment variable
NOSLOP_TASK_ACTION=done git commit -m "msg"
NOSLOP_TASK_ACTION=continue git commit -m "msg"
NOSLOP_TASK_ACTION=skip git commit -m "msg"
```

### Staged State Storage

Task completion intent stored in `.noslop/staged/task`:
```json
{"task_id": 1, "action": "done"}
```

Cleared by post-commit hook.

## Acceptance Criteria

- [ ] Pre-commit hook prompts for task status when task in progress
- [ ] User can choose done/continue/skip
- [ ] Done updates task file status
- [ ] Trailer added to commit message
- [ ] No prompt when no task in progress
- [ ] CI mode skips prompt (non-interactive)
- [ ] `NOSLOP_TASK_ACTION` env var works
- [ ] `noslop history` shows completions from git log
- [ ] Post-commit hook shows next ready task

## Edge Cases

1. **Multiple tasks in progress**: Only one task can be in_progress per branch. Prompt for that one.
2. **Commit fails after prompt**: Staged state preserved, re-applied on retry.
3. **Amend commits**: Re-prompt if task status might change.
4. **Merge commits**: Skip task prompt (merges aren't task work).
5. **Detached HEAD**: Skip task prompt (no branch = no tasks).

## Open Questions

1. Should "partial" update the task notes with commit summary?
2. Should we auto-unblock dependent tasks when parent completes?
3. Should there be a `--no-task` flag to skip task prompt entirely?
