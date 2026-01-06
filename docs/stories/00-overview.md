# noslop Feature Stories

This document outlines the planned evolution of noslop from a commit-time check tool to a complete agent autonomy platform.

## Vision

> **noslop: Autonomous agents that stay on the rails.**
>
> Define what needs to be done. Define what rules apply. Let the agent work. Trust the output.

## Core Value Proposition

| Component | Purpose |
|-----------|---------|
| **Checks** | Rules that get verified at commit time (enforcement) |
| **Tasks** | Work items scoped to branches (autonomy) |
| **UI** | Human visibility into agent work (trust) |

## Story Index

1. [Story 01: Rename Assert/Attest to Check/Verify](./01-rename-check-verify.md)
2. [Story 02: Branch-Scoped Tasks](./02-branch-scoped-tasks.md)
3. [Story 03: Commit-Time Task Completion](./03-commit-task-completion.md)
4. [Story 04: Sub-Branch Hierarchy](./04-sub-branch-hierarchy.md)
5. [Story 05: Local Web UI](./05-local-web-ui.md)
6. [Story 06: Agent Workflow Integration](./06-agent-workflow.md)

## Phased Rollout

| Phase | Stories | Milestone |
|-------|---------|-----------|
| v0.2 | 01 | Clean naming, better DX |
| v0.3 | 02, 03 | Branch-scoped task management |
| v0.4 | 04 | Hierarchical subtasks via sub-branches |
| v0.5 | 05 | Local UI for human oversight |
| v0.6 | 06 | Full agent autonomy workflow |
