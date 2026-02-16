# Setup Codebase Analysis

Deep analysis of the setup codebase (`/Users/saumil/Code/setup/`). This analysis informed the **agent integration layer** of noslop's review tool — specifically, the `AgentRuntime` trait for LLM-based review inference, the `noslop init <agent>` configuration generation, and the pipeline orchestrator pattern derived from ralph's iteration loop.

## Overview

**setup** is a Claude Code configuration toolkit providing autonomous iteration capabilities (ralph), git workflow tools (checkpoint, worktree), hooks, permissions, and global instructions. It's entirely shell-based with no compiled code.

## Project Structure

```
setup/
├── scripts/
│   ├── _lib.sh         # Shared utilities (77 lines)
│   ├── ralph           # Iteration loop (288 lines)
│   ├── checkpoint      # Safety commits (56 lines)
│   └── worktree        # Git worktree management (223 lines)
├── hooks/
│   ├── branch-protection.sh  # PreToolUse: blocks protected branches (17 lines)
│   └── auto-format.sh        # PostToolUse: ruff, prettier, rustfmt (35 lines)
├── commands/
│   ├── checkpoint.md   # /checkpoint slash command doc
│   ├── worktree.md     # /worktree slash command doc
│   └── ralph.md        # /ralph slash command doc
├── settings.json       # Claude Code settings (243 lines)
├── CLAUDE.md           # Repo documentation
├── CLAUDE.global.md    # Global instructions (symlinked to ~/.claude/CLAUDE.md)
├── install.sh          # Symlink installer (115 lines)
├── ralph-research-findings.md  # Agent harness research (82KB)
└── ralph-research-plan.md
```

## Components

### 1. Ralph — Iteration Loop (`scripts/ralph`)

**288 lines of bash**. Autonomous iteration loop for task completion.

#### Interface

```bash
ralph <plan_file> [max_iterations] [--worktree]
ralph plan.md                  # 20 iterations, current directory
ralph plan.md 50               # 50 iterations
ralph plan.md 20 --worktree    # Run in isolated git worktree
```

#### Core Algorithm

```
1. Parse arguments, resolve plan file to absolute path
2. If --worktree: create timestamped branch, copy plan, cd to worktree
3. For each iteration 1..max_iterations:
   a. Count tasks before (count_tasks from _lib.sh)
   b. Generate context (iteration N of M, git activity, plan status)
   c. If stuck_count >= 3: append stuck recovery section to prompt
   d. Invoke: claude -p "$FULL_PROMPT" --output-format text --permission-mode bypassPermissions
   e. Log output to .ralph-log
   f. Run checkpoint "ralph: iteration $i checkpoint"
   g. Check for <promise>COMPLETE</promise> → exit success
   h. Count tasks after, update stuck_count
   i. If stuck_count >= 5 → exit failure
4. Exit with "max iterations reached"
```

#### Context Generation

The `generate_context()` function builds per-iteration context:

- Iteration number and max
- Plan status (done/total from `count_tasks`)
- Recent git activity (last 5 commits via `git_activity`)

#### Prompt Template

Static template (lines 108-175) instructs Claude to:

1. Read the plan file
2. Find the first incomplete task
3. Orient: understand repo layout, run tests first
4. Implement: small atomic commits, one logical change per commit
5. Verify: run acceptance criteria, fix failures
6. Mark complete: update `- [ ]` to `- [x]` or `"passes": false` to `true`
7. Report: output `[RALPH:DONE task="..." iteration=N]`
8. If all done: output `<promise>COMPLETE</promise>`

Mentions `TeamCreate` for parallelization of independent subtasks.

#### Stuck Recovery

Appended when `stuck_count >= 3`:

- Instructs Claude to try a different approach
- Re-read task requirements
- Check test failures in detail
- Review previous iteration changes
- Skip blocked tasks, add notes
- Move to next task if stuck

#### Agent-Specific Elements

| Element                               | Claude-Specific? | Notes                    |
| ------------------------------------- | ---------------- | ------------------------ |
| `claude -p` invocation                | **Yes**          | CLI command              |
| `--output-format text`                | **Yes**          | Claude Code flag         |
| `--permission-mode bypassPermissions` | **Yes**          | Claude Code flag         |
| Prompt template structure             | No               | Could work for any agent |
| `<promise>COMPLETE</promise>` tag     | No               | Convention, not API      |
| TeamCreate reference                  | **Yes**          | Claude Code feature      |

### 2. Checkpoint (`scripts/checkpoint`)

**56 lines of bash**. Quick safety commits.

#### Interface

```bash
checkpoint                    # Message: "checkpoint: 2024-02-08 12:30:00"
checkpoint "work in progress" # Custom message
```

#### Behavior

- **Single-repo mode**: `git add -A && git commit -m "$MSG" --no-verify --quiet`
- **Multi-repo mode**: Iterates over all `*/` directories with `.git/`, checkpoints each

Uses `detect_repos` and `is_clean` from `_lib.sh`.

**Not agent-specific**. Pure git workflow tool.

### 3. Worktree (`scripts/worktree`)

**223 lines of bash**. Git worktree management.

#### Interface

```bash
worktree create <branch> [base]   # Create worktree(s)
worktree list                     # List active worktrees
worktree remove <branch>          # Remove worktree(s)
worktree clean                    # Remove all worktrees
worktree --repo <name> create ... # Scope to single repo (multi-repo only)
```

#### Worktree Location

Worktrees stored at: `../<repo-name>-worktrees/<branch>/`

Example: `/Users/saumil/Code/noslop/` creates worktrees at `/Users/saumil/Code/noslop-worktrees/<branch>/`

#### Features

- **Auto-detection**: Single-repo vs multi-repo mode
- **Branch management**: Creates new branch, or uses existing if branch exists
- **Cleanup**: Removes worktree, deletes merged branches, keeps unmerged
- **Prune**: `git worktree prune` before operations

**Not agent-specific**. Pure git workflow tool.

### 4. Shared Library (`scripts/_lib.sh`)

**77 lines of bash**. Utilities sourced by other scripts.

#### Functions

| Function         | Purpose                                      | Used By                     |
| ---------------- | -------------------------------------------- | --------------------------- |
| `detect_repos()` | Sets `MODE` (single/multi) and `REPOS` array | checkpoint, worktree, ralph |
| `count_tasks()`  | Count done/total tasks in plan file          | ralph                       |
| `git_activity()` | Recent commits from all repos                | ralph                       |
| `is_clean()`     | Check if working tree is clean               | checkpoint                  |

#### `count_tasks()` Implementation

Supports two formats:

- **Markdown**: `grep -cE '^\s*- \['` for total, `grep -cE '^\s*- \[x\]'` for done
- **JSON**: `grep -cE '"passes"\s*:'` for total, `grep -cE '"passes"\s*:\s*true'` for done

**Not agent-specific**. Pure utilities.

### 5. Hooks

#### `branch-protection.sh` (PreToolUse)

**17 lines**. Blocks Edit/Write on protected branches.

```bash
PROTECTED_BRANCHES=("main" "master" "development" "production")
# If current branch is protected, exit 2 to block
```

**Not agent-specific** — could work with any agent that uses Claude Code's hook system. However, the hook API format (exit codes, stderr messages) is Claude Code specific.

#### `auto-format.sh` (PostToolUse)

**35 lines**. Auto-formats after Edit/Write.

Routes by file extension:
| Extension | Formatter |
|-----------|-----------|
| `.py` | `ruff format` + `ruff check --fix` |
| `.ts`, `.tsx`, `.js`, `.jsx`, `.svelte`, `.css`, `.html`, `.json`, `.md`, `.yaml`, `.yml` | `npx prettier --write` |
| `.rs` | `rustfmt` |

Reads tool input JSON from stdin: `jq -r '.tool_input.file_path // empty'`

**Partially agent-specific**. The file formatting is universal. The input format (JSON via stdin) is Claude Code specific.

### 6. Settings.json

**243 lines**. Claude Code global configuration.

#### Key Sections

**Model**: `"model": "opus"`

**Attribution**:

```json
"attribution": {
  "commit": "Co-Authored-By: Claude <noreply@anthropic.com>",
  "pr": "Generated with [Claude Code](https://claude.ai/claude-code)"
}
```

**Permissions** (97 allow rules):

- Core tools: Read, Edit, Write, Glob, Grep, WebFetch, WebSearch
- Git: status, diff, log, branch, show, remote, rev-parse, add, commit, checkout, switch, stash, merge, rebase, fetch, cherry-pick, tag
- Node: npm run/install/test/ci/exec, npx, node
- Python: poetry (all), python -m, pip install/list
- Docker: compose, ps, logs, images, inspect, exec
- Utils: gh, ls, pwd, which, cat, head, tail, wc, find, tree, mkdir, cp, mv, touch, sort, uniq, grep, rg, sed, awk, jq, curl, echo, printf, date, env
- Tools: alembic, ruff, mypy, eslint, prettier, tsc, pre-commit
- Custom: checkpoint, worktree, ralph

**Deny rules**:

- Secrets: `.env`, `.env.*`, `secrets/*.json`, `~/.ssh/**`, `ejson-keys/**`
- Destructive: `rm -rf /`, `git push --force`, `git reset --hard`, `git clean -f`, `chmod 777`, disk writes

**Ask rules**: `git push`, `docker run/build`, `rm -rf/-r`

**Hooks**:
| Event | Matcher | Hook |
|-------|---------|------|
| PreToolUse | Edit\|Write | branch-protection.sh |
| PostToolUse | Edit\|Write | auto-format.sh |
| PostToolUse | Bash | Command logging to ~/.claude/command-log.txt |
| Stop | _ | LLM prompt to verify task completion |
| Notification | _ | Desktop notification (notify-send) |

**Sandbox**:

- Enabled with auto-allow
- Network restricted to: GitHub, npm, PyPI, Docker Hub, Stack Overflow, MDN, docs sites
- Docker excluded (incompatible)

**Feature flags**:

- `alwaysThinkingEnabled: true`
- `teammateMode: "auto"`
- `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: "1"`

**Entirely Claude-specific**. The settings.json format, hook system, permission syntax, and sandbox configuration are all Claude Code APIs.

### 7. CLAUDE.global.md

**44 lines**. Global instructions symlinked to `~/.claude/CLAUDE.md`.

Content:

- Lists available tools (checkpoint, worktree, ralph)
- Usage examples for each
- Coding conventions:
  - Git flow (feature branches)
  - Run checkpoint before risky ops
  - No "enhanced/extended/improved" prefixes
  - Type hints for Python
  - TypeScript strict mode

**Claude-specific in location** (`~/.claude/CLAUDE.md`) but the content is general development guidance.

### 8. Slash Commands (`commands/*.md`)

Slash command definitions for Claude Code's `/command` system.

#### `/checkpoint` (checkpoint.md)

32 lines. Instructs Claude when to run `checkpoint`:

- Before: deletions, large refactors, config changes, resets, unfamiliar commands, switching approaches
- Between: major implementation steps, feature completion, optimization attempts
- Not after: read-only ops, minor edits, normal commits

#### `/worktree` (worktree.md)

30 lines. Documents commands and when to suggest worktrees.

#### `/ralph` (ralph.md)

169 lines. Complex slash command for launching ralph with live monitoring.

Steps:

1. Detect repo layout (single vs multi)
2. Count initial plan status
3. Launch ralph in background (`run_in_background: true`)
4. Monitor loop (30s intervals):
   - Sleep 30s
   - Gather status (commits, plan, current task, WIP)
   - Check TaskOutput (non-blocking)
   - Display progress report
5. Final report on completion

**Claude-specific**. The `/command` system is Claude Code only.

### 9. Install Script (`install.sh`)

**115 lines**. Sets up symlinks and PATH.

Creates:

- `~/.claude/settings.json` → `./settings.json`
- `~/.claude/hooks/*.sh` → `./hooks/*.sh`
- `~/.claude/scripts/*` → `./scripts/*`
- `~/.claude/commands/*.md` → `./commands/*.md`
- `~/.claude/CLAUDE.md` → `./CLAUDE.global.md`

Adds `~/.claude/scripts` to PATH via `.zshrc` or `.bashrc`.

**Claude-specific in target location** (`~/.claude/`).

## Agent-Specific vs Agent-Agnostic Summary

### Agent-Agnostic Components

These can be migrated directly to noslop as Rust code:

| Component        | Lines | Migration Notes                                             |
| ---------------- | ----- | ----------------------------------------------------------- |
| `checkpoint`     | 56    | Use git2 crate. Multi-repo detection reusable.              |
| `worktree`       | 223   | Use git2 crate for worktree ops. Same interface.            |
| `_lib.sh` (most) | ~60   | `detect_repos`, `is_clean`, `count_tasks` translate to Rust |
| `count_tasks`    | ~20   | Regex-based task counting, trivial in Rust                  |

### Agent-Specific Components (Claude)

These define what `noslop init claude` must generate:

| Component          | Purpose                        | Migration               |
| ------------------ | ------------------------------ | ----------------------- |
| `settings.json`    | Claude Code config             | Generate on init        |
| `CLAUDE.global.md` | Global instructions            | Generate on init        |
| `hooks/*.sh`       | PreToolUse/PostToolUse         | Generate on init        |
| Slash commands     | /checkpoint, /worktree, /ralph | Generate on init        |
| `ralph` invocation | `claude -p` with flags         | AgentRuntime trait impl |

### Iteration Loop (ralph) — Hybrid

The ralph script has both agent-agnostic and agent-specific parts:

**Agent-agnostic** (port to core noslop):

- Iteration loop structure
- Plan file parsing (MD + JSON)
- Task counting
- Stuck detection (3/5 iteration thresholds)
- Context generation
- Progress file management
- Checkpoint integration
- Worktree integration

**Agent-specific** (AgentRuntime trait):

- Invocation command: `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`
- Output parsing (completion tag, status line)
- Tool references (TeamCreate)

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                          User runs ralph                         │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│  setup/scripts/ralph                                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Parse args, resolve plan file, create worktree if needed │   │
│  └────────────────────────────────┬─────────────────────────┘   │
│                                   │                             │
│  ┌────────────────────────────────▼─────────────────────────┐   │
│  │ For each iteration:                                       │   │
│  │   1. count_tasks (before)                                 │   │
│  │   2. generate_context                                     │   │
│  │   3. build prompt (template + context + stuck if needed)  │   │
│  │   4. claude -p "$FULL_PROMPT" ──────────────────────────────►│
│  │   5. log output                                     ◄───────│ Claude
│  │   6. checkpoint "ralph: iteration N checkpoint"            │   │
│  │   7. check <promise>COMPLETE</promise>                    │   │
│  │   8. count_tasks (after), update stuck_count              │   │
│  │   9. exit if stuck >= 5 or complete                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Inputs: plan_file, max_iterations, --worktree                  │
│  Outputs: .ralph-log, git commits, updated plan file            │
└─────────────────────────────────────────────────────────────────┘
```

## Research Findings Integration

The `ralph-research-findings.md` (82KB) documents:

1. **Anthropic patterns**: Two-agent harness, progress files, JSON plans, strong constraints, Puppeteer MCP
2. **Claude Agent SDK**: `query()`, hooks, custom tools, subagents, session resume
3. **Competitor architectures**: Aider (architect/editor), SWE-agent, OpenHands, Cursor, Devin, SICA

Key recommendations for ralph migration:

- Add initializer phase (first iteration gets setup-focused prompt)
- Create structured progress file (not just git history)
- Prefer JSON plans over markdown
- Use SDK's `query()` for structured messages and cost tracking
- Add custom MCP tools for reliable plan updates
- Consider session resume for multi-turn within iterations
- Use `max_budget_usd` and `max_turns` for safety

## Dependencies

### Shell Requirements

- bash with `set -euo pipefail`
- jq (for hook input parsing)
- git (for all operations)

### Formatters (optional, for auto-format hook)

- ruff (Python)
- npx prettier (JS/TS/etc)
- rustfmt (Rust)

### Claude Code Requirements

- claude CLI in PATH
- `~/.claude/` directory structure
- Settings.json hook support

## What Transfers to Review Tool

These setup concepts carry over to noslop's review architecture, reframed for code review instead of code generation:

| Setup Concept                        | Review Tool Role                                              | Notes                                                                                                                                                                                            |
| ------------------------------------ | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **AgentRuntime concept**             | Stays — powers `AgentAnalyzer` for LLM-based review inference | Same trait shape: invoke agent, parse output. Different prompts (review instead of generation).                                                                                                  |
| **Agent CLI invocation patterns**    | Same mechanism for calling Claude/Codex/etc.                  | `claude -p` with review prompts instead of generation prompts. Flags, output parsing, permission handling all transfer.                                                                          |
| **Settings/hooks/config generation** | `noslop init <agent>` still generates agent-specific config   | Same install.sh pattern: settings.json, CLAUDE.md, hooks. Content changes but mechanism is identical.                                                                                            |
| **Ralph loop concept**               | Becomes the `ReviewPipeline` orchestrator                     | Ralph iterates over plan tasks; `ReviewPipeline` iterates over analyzers. Same shape: ordered iteration, context accumulation, result aggregation.                                               |
| **Context generation**               | Analyzers receive prior findings as context                   | Ralph's `generate_context()` builds per-iteration context from git history. Pipeline builds per-analyzer context from prior findings. Same principle: each step is informed by what came before. |
| **Research findings**                | Applicable to review prompts                                  | Prompt caching, structured output, cost tracking (`max_budget_usd`), SDK `query()` — all apply to LLM review inference. Competitor analysis (Aider, SWE-agent) informs review prompt design.     |
| **Auto-format hook**                 | Becomes a built-in `FormattingAnalyzer` or script analyzer    | Instead of formatting after edits, detects formatting issues in diffs before push.                                                                                                               |
| **Branch protection hook**           | Policy enforcement in review pipeline                         | Could become a built-in analyzer that checks branch naming, protected paths, etc.                                                                                                                |

## What Drops

These setup concepts do not transfer to the review tool:

| Setup Concept                                       | Why It Drops                                                                                                         |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Plan file parsing** (MD + JSON task formats)      | No plans in review — the pipeline operates on diffs, not task lists                                                  |
| **Stuck detection** (3/5 iteration thresholds)      | Not applicable — analyzers either produce findings or they don't; there is no "stuck" state                          |
| **Code generation prompts**                         | Review prompts replace them — the LLM reads code, not writes it                                                      |
| **Checkpoint as primary feature**                   | Remains a utility script in PATH, but not a noslop command — review does not generate code that needs safety commits |
| **Worktree for parallel coding**                    | Not needed for review — review runs in the current worktree against the current diff                                 |
| **Multi-repo mode** (`detect_repos`)                | Already dropped in earlier planning                                                                                  |
| **count_tasks / git_activity**                      | Plan-specific utilities with no review equivalent                                                                    |
| **Slash commands** (/checkpoint, /worktree, /ralph) | Claude Code specific; not part of the review tool itself                                                             |
| **Progress file management**                        | No long-running iterations to track — pipeline runs once and produces findings                                       |

## Summary

The setup repo is a Claude Code configuration toolkit. For the review tool direction, it informs the **agent integration layer** — how noslop invokes LLMs for review inference and how `noslop init <agent>` generates agent-specific configuration.

| Category           | Transfers to Review Tool                                | Drops                                                |
| ------------------ | ------------------------------------------------------- | ---------------------------------------------------- |
| Agent invocation   | `AgentRuntime` trait, CLI patterns, output parsing      | —                                                    |
| Configuration      | `noslop init <agent>`, settings/hooks/config generation | —                                                    |
| Loop/orchestration | Pipeline orchestrator pattern (from ralph)              | Plan parsing, stuck detection, progress files        |
| Git workflow       | checkpoint/worktree remain as PATH utilities            | Not noslop commands; not part of review              |
| Prompts            | Prompt structure, caching, cost tracking                | Code generation prompts (replaced by review prompts) |
| Research           | Competitor patterns, SDK integration, prompt design     | —                                                    |
| Slash commands     | —                                                       | Claude Code specific, not part of review tool        |

The review tool strategy for setup's contributions:

1. Define `AgentRuntime` trait for LLM review inference (from ralph's invocation pattern)
2. Define `AgentConfig` trait for config generation (from install.sh/settings.json pattern)
3. Implement Claude adapter for `noslop init claude` (generates settings.json, CLAUDE.md, hooks)
4. Build `ReviewPipeline` orchestrator using ralph's iteration-with-context pattern
5. Keep checkpoint and worktree as standalone PATH utilities (not noslop subcommands)
