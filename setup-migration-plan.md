# Setup → Noslop Migration: Implementation Plan

Ralph will analyze both codebases and produce a detailed implementation plan
for absorbing the `setup` repo into `noslop`, with an agent-agnostic abstraction
layer. Initial agent support targets **Claude** and **Codex** only; the trait
design must allow adding more agents later without changing core code.

## Context

- **noslop** (`/Users/saumil/Code/noslop`): Rust CLI with hexagonal architecture.
  Pre-commit checks, acknowledgment tracking, code reviews, Tauri+Svelte GUI.
- **setup** (`/Users/saumil/Code/setup`): Claude Code config toolkit. Contains
  `ralph` (iteration loop), `checkpoint`, `worktree`, hooks, settings, installer.
- **Goal**: noslop becomes the single tool for AI-assisted development — conventions
  (check/ack), process (run/checkpoint/worktree), and safety (hooks/review).
- **Key design decision**: `noslop init <agent>` abstracts over which AI agent the
  user works with. The iteration loop, worktree, checkpoint, and checks all work
  regardless of agent choice.
- **Scope**: Claude and Codex adapters only. The trait is generic enough for
  future agents (Cursor, Aider, etc.) but we only implement two now.

## Research references

- Setup research findings: `/Users/saumil/Code/setup/ralph-research-findings.md`
- Anthropic agent harness patterns, Claude Agent SDK, competitor architectures
  (Aider, SWE-agent, OpenHands, Cursor, Devin, SICA) are all documented there.

## Tasks

- [x] **1. Deep-analyze noslop codebase**: Read every source file in `src/`. Document the hexagonal architecture (ports, adapters, core services), all CLI commands, all models, the test patterns used (unit tests, integration tests, test fixtures), code conventions (clippy settings, error handling, module structure), and the existing hook installation system. Write findings to `migration/01-noslop-analysis.md`.

- [ ] **2. Deep-analyze setup codebase**: Read every file in `/Users/saumil/Code/setup/`. Document each component (ralph, checkpoint, worktree, \_lib.sh, hooks, settings.json, CLAUDE.global.md, install.sh, slash commands), their interfaces, dependencies, and behaviors. Identify what is agent-specific (Claude-only) vs agent-agnostic. Write findings to `migration/02-setup-analysis.md`.

- [ ] **3. Design the Agent port trait**: Based on both analyses, design the `AgentConfig` and `AgentRuntime` port traits that go in `src/core/ports/agent.rs`. Define every method signature with Rust types. Consider: what does init need from each agent? What does `noslop run` need? How does output parsing differ? The trait must be generic enough for future agents but only Claude and Codex are implemented now. Follow noslop's existing port patterns (see `vcs.rs`, `check_repo.rs`). Write the trait design to `migration/03-agent-port-design.md`.

- [ ] **4. Design the Claude and Codex adapters**: For Claude: specify what config files are generated (settings.json, CLAUDE.md, hooks), how invocation works (`claude -p` then Agent SDK), how output is parsed. For Codex: specify config files, invocation (`codex` CLI), output parsing. Show the adapter struct and impl blocks for both. Detail what Claude's adapter generates by extracting from setup's actual files. Write to `migration/04-agent-adapters-design.md`.

- [ ] **5. Design checkpoint and worktree as noslop subcommands**: Map the bash implementations (`checkpoint` ~56 lines, `worktree` ~223 lines, `_lib.sh` ~77 lines) to Rust equivalents using noslop's existing `git2` dependency and adapter patterns. Define the CLI interface (`noslop checkpoint [msg]`, `noslop worktree create|list|remove|clean`). Show how they integrate with the existing `cli/app.rs` command enum. Write to `migration/05-checkpoint-worktree-design.md`.

- [ ] **6. Design `noslop run` (the iteration loop)**: Translate ralph's bash loop into a Rust orchestrator design. Cover: plan file parsing (JSON + Markdown), progress file management, failure memory, stuck detection (graduated 4-level escalation), post-iteration test gates, post-loop review, and the agent invocation point. Show how `noslop run` calls the `AgentRuntime` trait. Incorporate the quick wins from setup's research findings. Write to `migration/06-run-loop-design.md`.

- [ ] **7. Design `noslop init <agent>` expansion**: Show how the current `init.rs` command evolves. The bare `noslop init` still works (git hooks + .noslop.toml only). `noslop init claude` and `noslop init codex` add agent-specific config. Define the `[agent]` section in `.noslop.toml`. Show the CLI changes in `app.rs`. Write to `migration/07-init-expansion-design.md`.

- [ ] **8. Design the config evolution**: Show the full `.noslop.toml` schema with agent config, loop settings, and existing check definitions. Define any new files (.noslop/progress.md, .noslop/failures.jsonl, etc.) and their formats. Write to `migration/08-config-design.md`.

- [ ] **9. Design the test strategy**: For each new module, specify: what unit tests (with mocks via mockall), what integration tests, what test fixtures. Follow noslop's existing test patterns exactly (see tests in checker.rs, matcher.rs, etc.). Cover: agent trait mocking, checkpoint/worktree testing with tempdir git repos, plan parsing tests, stuck detection tests. Write to `migration/09-test-strategy.md`.

- [ ] **10. Produce the final implementation plan**: Synthesize all design documents into a single ordered implementation plan with: phases, file-by-file changes, dependency order, estimated scope per phase, and migration path (how to deprecate the setup repo). Include a Cargo.toml dependency analysis. Write to `migration/10-implementation-plan.md`.
