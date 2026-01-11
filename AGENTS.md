# AGENTS.md - Instructions for AI Agents

This repository uses **noslop** for task management and verification tracking. All AI agents working on this codebase MUST use noslop to coordinate work.

## Required: Check for Tasks Before Starting

Before doing any work, always check for existing tasks:

```bash
noslop status          # Quick overview
noslop task list       # All tasks with status
noslop task next       # Get next prioritized task
```

## Required: Track Your Work

1. **Start a task** before beginning work:

   ```bash
   noslop task start TSK-N
   # Or get the next one automatically:
   noslop task next --start
   ```

2. **Complete the task** when done:

   ```bash
   noslop task done
   ```

3. **Add new tasks** when you discover work:

   ```bash
   noslop task add "Description of work" --priority p1
   ```

## Task Priorities

- `p0` - Critical: Blocking issues, security vulnerabilities
- `p1` - High: Important features, significant bugs
- `p2` - Medium: Normal priority work (default)
- `p3` - Low: Nice-to-have, minor improvements

## Machine-Readable Output

For parsing output programmatically, use JSON mode:

```bash
export NOSLOP_JSON=1
# Or use --json flag:
noslop --json task list
noslop --json status
```

## Task Dependencies

Tasks can be blocked by other tasks:

```bash
# Block TSK-2 until TSK-1 is done
noslop task block TSK-2 --by TSK-1

# See only unblocked tasks ready for work
noslop task list --unblocked
```

## Web UI (Optional)

For visual task management:

```bash
noslop ui --port 9999
```

## Check System

When modifying files, checks may apply:

```bash
noslop check list                    # See checks
noslop check run                     # Run checks on staged files
noslop verify CHK-N -m "Explanation" # Verify a check
```

## Integration with Commits

noslop integrates with git hooks:

- Pre-commit: Runs checks, prompts for task status
- Prepare-commit-msg: Adds verification trailers
- Post-commit: Clears staged verifications

The current task is automatically referenced in commits.
