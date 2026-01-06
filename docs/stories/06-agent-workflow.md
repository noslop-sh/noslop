# Story 06: Agent Workflow Integration

## Summary

Define how AI coding agents interact with noslop for autonomous work. Agents query for tasks, create sub-branches, work focused, and commit through the check/verify flow.

## Motivation

**Current problem:**
- Agents plan internally, plans disappear between sessions
- No structured way for agents to pick up work
- Humans must direct each step

**Solution:**
- Agent queries `noslop task next` for work
- Agent creates sub-branches for focused implementation
- Commit hook enforces quality (can't skip checks)
- Task completion recorded automatically
- Human sets up work, agent executes autonomously

## User Stories

### US-06.1: Agent gets next task
**As an** AI agent
**I want** to query for the next unblocked task
**So that** I know what to work on

```bash
$ noslop task next --json
{
  "id": 2,
  "title": "Add refresh token logic",
  "branch": "feature/oauth",
  "notes": "Handle token expiry and refresh flow",
  "parent_notes": "See RFC: docs/rfcs/oauth.md"
}
```

### US-06.2: Agent starts task with sub-branch
**As an** AI agent
**I want** to start a task in an isolated sub-branch
**So that** I can work focused without affecting parent

```bash
$ noslop task start 2 --branch --json
{
  "success": true,
  "task_id": 2,
  "branch": "feature/oauth/refresh-tokens",
  "message": "Created sub-branch and started task"
}
```

### US-06.3: Agent commits through checks
**As an** AI agent
**I want** commits to go through check verification
**So that** I don't skip quality gates

```bash
$ git commit -m "Add refresh token handling"

# Check runs automatically via hook
# Agent must have verified checks or commit fails

$ echo $?
0  # Success - all checks verified
```

### US-06.4: Agent marks task done at commit
**As an** AI agent
**I want** to mark tasks done via environment variable
**So that** I don't need interactive prompts

```bash
$ NOSLOP_TASK_ACTION=done git commit -m "Complete refresh token logic"

✓ Committed
✓ Task 2 marked done
```

### US-06.5: Agent merges back to parent
**As an** AI agent
**I want** to merge my sub-branch back to parent
**So that** my work is integrated

```bash
$ noslop branch done --json
{
  "success": true,
  "merged": "feature/oauth/refresh-tokens",
  "into": "feature/oauth",
  "next_task": {
    "id": 3,
    "title": "Add integration tests"
  }
}
```

### US-06.6: Agent continues with next task
**As an** AI agent
**I want** to immediately get the next task after completing one
**So that** I can work continuously

```bash
$ noslop task next --start --branch --json
{
  "success": true,
  "task_id": 3,
  "title": "Add integration tests",
  "branch": "feature/oauth/tests"
}

# Now on feature/oauth/tests, task 3 in progress
```

### US-06.7: Session start context
**As an** AI agent
**I want** to get context when starting a session
**So that** I understand the project state

```bash
$ noslop status --json
{
  "branch": "feature/oauth",
  "in_progress_task": null,
  "ready_tasks": [
    {"id": 1, "title": "Implement OAuth routes"}
  ],
  "checks": [
    {"id": "NOS-1", "target": "src/auth/**", "message": "Security review"}
  ]
}
```

## Technical Design

### Agent Configuration (CLAUDE.md / AGENTS.md)

```markdown
## Task Management

This project uses noslop for task tracking and commit checks.

### Session Start
Run `noslop status --json` to see:
- Current branch and in-progress task
- Ready tasks (unblocked)
- Applicable checks

### Working on Tasks
1. Get next task: `noslop task next --json`
2. Start with sub-branch: `noslop task start <id> --branch`
3. Work and commit normally
4. Mark done: `NOSLOP_TASK_ACTION=done git commit -m "..."`
5. Merge back: `noslop branch done`
6. Repeat with next task

### Commit Requirements
All commits go through noslop checks. Verify checks before committing:
- `noslop check run` to see pending checks
- `noslop verify <id> -m "reason"` to verify

### Environment Variables
- `NOSLOP_TASK_ACTION=done|continue|skip` - non-interactive task completion
- `NOSLOP_JSON=1` - JSON output for all commands
```

### JSON Output Mode

All commands support `--json` for agent consumption:

```bash
$ noslop task list --json
{
  "branch": "feature/oauth",
  "tasks": [
    {"id": 1, "title": "...", "status": "done"},
    {"id": 2, "title": "...", "status": "in_progress"},
    {"id": 3, "title": "...", "status": "pending", "blocked_by": [2]}
  ]
}

$ noslop check run --json
{
  "passed": false,
  "blocking": [
    {"id": "NOS-1", "file": "src/auth/oauth.py", "message": "Security review"}
  ],
  "verified": []
}

$ noslop verify NOS-1 -m "Reviewed" --json
{
  "success": true,
  "check_id": "NOS-1",
  "message": "Reviewed"
}
```

### Environment Variables

| Variable | Values | Effect |
|----------|--------|--------|
| `NOSLOP_TASK_ACTION` | `done`, `continue`, `skip` | Skip interactive task prompt |
| `NOSLOP_JSON` | `1` | JSON output for all commands |
| `NOSLOP_CI` | `1` | CI mode (strict, non-interactive) |

### Complete Agent Loop

```python
# Pseudocode for agent workflow

def work_session():
    # 1. Get status
    status = run("noslop status --json")

    while True:
        # 2. Get next task
        task = run("noslop task next --json")
        if not task:
            break  # All done

        # 3. Start task with sub-branch
        run(f"noslop task start {task['id']} --branch")

        # 4. Read task context
        context = run(f"noslop task show {task['id']} --json")

        # 5. Plan and implement
        plan = create_plan(task, context)

        for step in plan:
            implement(step)

            # 6. Check before commit
            checks = run("noslop check run --json")
            for check in checks['blocking']:
                verify_check(check)

            # 7. Commit with task action
            is_last_step = (step == plan[-1])
            action = "done" if is_last_step else "continue"
            run(f"NOSLOP_TASK_ACTION={action} git commit -m '{step['message']}'")

        # 8. Merge back to parent
        run("noslop branch done")

def verify_check(check):
    # Actually verify (run command, review code, etc.)
    verification = do_verification(check)
    run(f"noslop verify {check['id']} -m '{verification}'")
```

### Hook Behavior with Agents

Pre-commit hook detects agent context:
1. If `NOSLOP_TASK_ACTION` set → use that, no prompt
2. If `NOSLOP_CI=1` → fail on unverified checks, no prompt
3. Otherwise → interactive prompts

## Acceptance Criteria

- [ ] All commands support `--json` flag
- [ ] `NOSLOP_TASK_ACTION` env var works
- [ ] `noslop task next` returns unblocked task
- [ ] `noslop task start --branch` creates sub-branch
- [ ] `noslop branch done` merges and gets next
- [ ] Agent can complete full loop without interaction
- [ ] Checks still enforce quality (can't skip)

## Integration with Claude Code

```markdown
# CLAUDE.md

## Using noslop

This project uses noslop. At session start:

1. Run `noslop status --json` to understand current state
2. Run `noslop task next` to see what's ready

When working:
- Create sub-branches for focused work
- Verify checks before committing
- Use `NOSLOP_TASK_ACTION=done` when task is complete

Commands:
- `noslop task next` - get next unblocked task
- `noslop task start <id> --branch` - start with sub-branch
- `noslop check run` - see pending checks
- `noslop verify <id> -m "..."` - verify a check
- `noslop branch done` - merge to parent, get next task
```

## Open Questions

1. Should agent auto-verify checks if `run` command passes?
2. Should there be a `noslop agent loop` command that does the full workflow?
3. How to handle agent errors mid-task? (Partial commits, recovery)
4. Should we track which agent worked on which task? (Already in trailers via `attested_by`)
