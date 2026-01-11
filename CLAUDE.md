# CLAUDE.md - Instructions for Claude Code

This repository uses **noslop** for task management. Before starting any work, check for existing tasks and use noslop to track your progress.

## Getting Started

```bash
# Check current status (tasks, checks, branch)
noslop status

# List all pending tasks
noslop task list

# Get the next task to work on
noslop task next --start

# When done with current task
noslop task done
```

## Task Workflow

1. **Before starting work**: Run `noslop task list` to see what needs to be done
2. **Pick a task**: Run `noslop task start TSK-N` or `noslop task next --start`
3. **While working**: The current task is tracked automatically
4. **When complete**: Run `noslop task done` to mark it complete

## Adding New Tasks

When you identify work that needs to be done:

```bash
# Add a task with priority
noslop task add "Implement feature X" --priority p1

# Priorities: p0 (critical), p1 (high), p2 (medium), p3 (low)
```

## JSON Output for Parsing

All commands support `--json` flag or `NOSLOP_JSON=1` env var:

```bash
noslop --json task list
noslop --json status
```

## Build Commands

```bash
cargo build --release --features ui
cargo test --features ui
cargo fmt --all
cargo clippy --features ui -- -D warnings
```

## Pre-commit Checks

This repo uses noslop's check system. When you modify files, checks may need verification:

```bash
# See active checks
noslop check list

# Verify a check (proves you considered it)
noslop verify CHK-N --message "Verified: explanation"
```
