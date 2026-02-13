# Noslop Codebase Analysis

Deep analysis of the noslop codebase for the setup migration project.

## Overview

**noslop** is a Rust CLI tool for enforcing code review considerations via pre-commit hooks. It follows a hexagonal (ports & adapters) architecture with strict separation between business logic and I/O.

**Version**: 0.1.2
**Rust Edition**: 2024
**Architecture**: Hexagonal (Ports & Adapters)

## Project Structure

```
noslop/
├── src/
│   ├── main.rs           # Binary entry point
│   ├── lib.rs            # Library crate with re-exports
│   ├── cli/              # CLI layer
│   │   ├── app.rs        # Clap definitions, command dispatch
│   │   └── commands/     # Command implementations
│   ├── core/             # Pure domain logic (no I/O)
│   │   ├── models/       # Domain types
│   │   ├── ports/        # Trait interfaces
│   │   └── services/     # Business logic
│   ├── adapters/         # I/O implementations
│   │   ├── git/          # Git operations
│   │   ├── toml/         # .noslop.toml parsing
│   │   ├── file/         # JSON file storage
│   │   ├── trailer/      # Commit trailer storage
│   │   └── review.rs     # Review JSON storage
│   ├── shared/           # Cross-cutting utilities
│   │   ├── parser/       # Token/pattern parsing
│   │   └── resolver.rs   # File resolution
│   ├── storage/          # Storage abstraction facade
│   ├── output.rs         # Human/JSON output formatting
│   ├── git/              # Binary-side git re-exports
│   └── noslop_file.rs    # Binary-side toml re-exports
├── tests/
│   ├── common/           # Test utilities
│   ├── unit/             # Unit tests
│   ├── adapter/          # Adapter tests
│   └── integration/      # Integration tests
└── gui/                  # Tauri + Svelte GUI (separate)
```

## Hexagonal Architecture

### Core Layer (`src/core/`)

Pure domain logic with **no I/O dependencies**. All external interactions are abstracted through port traits.

```
┌─────────────────────────────────────┐
│           Adapters (I/O)            │
│  ┌─────┐  ┌─────┐  ┌─────────────┐  │
│  │TOML │  │ Git │  │   Trailer   │  │
│  └──┬──┘  └──┬──┘  └──────┬──────┘  │
└─────┼───────┼─────────────┼─────────┘
      │       │             │
┌─────▼───────▼─────────────▼─────────┐
│              Ports (Traits)          │
│  CheckRepository                     │
│  AcknowledgmentStore                 │
│  VersionControl                      │
│  ReviewStore                         │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│           Core (Pure Logic)         │
│  ┌──────────┐  ┌────────────────┐   │
│  │  Models  │  │    Services    │   │
│  │  Check   │  │  check_items   │   │
│  │  Ack     │  │ matches_target │   │
│  │ Target   │  └────────────────┘   │
│  │ Severity │                       │
│  └──────────┘                       │
└─────────────────────────────────────┘
```

### Models (`src/core/models/`)

| Model | File | Description |
|-------|------|-------------|
| `Check` | `check.rs` | "When this code changes, verify this" - ID, target, message, severity |
| `Acknowledgment` | `acknowledgment.rs` | Proof that a check was considered - check_id, message, acknowledged_by |
| `Severity` | `severity.rs` | Enum: `Info`, `Warn`, `Block` |
| `Target` | `target.rs` | Parsed target reference with `PathSpec` and `Fragment` |
| `Review` | `review.rs` | Code review session with comments on a commit range |
| `ReviewComment` | `review.rs` | Check + diff position + resolution status |

**Target Parsing** supports:
- Exact paths: `src/auth.rs`
- Glob patterns: `*.rs`, `src/**/*.rs`
- Fragment identifiers: `#L42`, `#L42-L50`, `#Session`

### Ports (`src/core/ports/`)

| Trait | File | Methods |
|-------|------|---------|
| `CheckRepository` | `check_repo.rs` | `find_for_files`, `add`, `remove`, `list`, `list_filtered` |
| `AcknowledgmentStore` | `acknowledgment_store.rs` | `stage`, `staged`, `clear_staged`, `format_trailers`, `parse_from_commit` |
| `VersionControl` | `vcs.rs` | `staged_files`, `repo_name`, `repo_root`, `install_hooks`, `is_inside_repo`, `current_branch` |
| `ReviewStore` | `review_store.rs` | `save`, `load`, `list_all`, `list_open`, `delete`, `find_blocking_for_file` |

**Key Design Patterns**:
- All traits are `Send + Sync` for thread safety
- Traits are annotated with `#[cfg_attr(test, mockall::automock)]` for auto-generated mocks
- Return `anyhow::Result` for error handling

### Services (`src/core/services/`)

| Service | File | Description |
|---------|------|-------------|
| `check_items` | `checker.rs` | Pure function: checks + acks + file_count → CheckResult |
| `matches_target` | `matcher.rs` | Pattern matching: glob patterns, exact paths, recursive globs |

**`CheckResult`** structure:
- `passed: bool` - No blocking unacknowledged checks
- `files_checked: usize`
- `blocking: Vec<CheckItemResult>` - Need acknowledgment
- `warnings: Vec<CheckItemResult>` - Shown but don't block
- `acknowledged: Vec<CheckItemResult>` - Already acknowledged

### Adapters (`src/adapters/`)

| Adapter | File | Implements |
|---------|------|------------|
| `TomlCheckRepository` | `toml/repository.rs` | `CheckRepository` - reads `.noslop.toml` files |
| `GitVersionControl` | `git/mod.rs` | `VersionControl` - git operations via `Command` |
| `TrailerAckStore` | `trailer/mod.rs` | `AcknowledgmentStore` - commit trailers |
| `FileStore` | `file/mod.rs` | Staging storage in `.noslop/staged-acks.json` |
| `FileReviewStore` | `review.rs` | `ReviewStore` - JSON files in `.noslop/reviews/` |

**Git Hooks** (`adapters/git/hooks.rs`):
- `pre-commit`: Runs `noslop check`
- `commit-msg`: Runs `noslop add-trailers "$1"`
- `post-commit`: Runs `noslop clear-staged`

Hook installation:
- Creates hooks in `.git/hooks/`
- Appends to existing hooks if present
- Sets executable permissions on Unix
- `remove_noslop_hooks()` safely removes noslop content while preserving other content

## CLI Commands

Defined in `src/cli/app.rs`:

| Command | Description |
|---------|-------------|
| `noslop init [--force]` | Initialize noslop in repo |
| `noslop check [--ci]` | Validate checks for staged changes |
| `noslop check add <target> -m <msg> [-s <sev>]` | Add a check |
| `noslop check list [--target <filter>]` | List checks |
| `noslop check remove <id>` | Remove a check |
| `noslop ack <id> -m <msg>` | Acknowledge a check |
| `noslop add-trailers <file>` | (hidden) Hook: add trailers |
| `noslop clear-staged` | (hidden) Hook: clear staged acks |
| `noslop review start <base> <head>` | Start code review |
| `noslop review comment <id> <target> -m <msg>` | Add comment |
| `noslop review list [--open]` | List reviews |
| `noslop review show <id>` | Show review details |
| `noslop review resolve <comment_id> [-m <msg>]` | Resolve comment |
| `noslop review close <id>` | Close review |
| `noslop version` | Show version |

**Output Modes**:
- `--json` flag for machine-readable output
- `--verbose` flag for debug logging

## Configuration

### `.noslop.toml` Schema

```toml
[project]
prefix = "NOS"   # 3-letter prefix for check IDs

[[check]]
id = "NOS-1"                              # Optional (auto-generated if omitted)
target = "src/adapters/git/hooks.rs"      # Path or glob pattern
message = "Verify hook installation"       # Check message
severity = "block"                         # info | warn | block
tags = ["security"]                        # Optional tags
```

Features:
- Multiple `.noslop.toml` files allowed (hierarchical, root-first)
- Auto-generated JIRA-style IDs: `{PREFIX}-{N}`
- Glob patterns: `*.rs`, `src/**/*.rs`, `src/*.rs`
- Fragment targeting: `src/auth.rs#L42`, `src/auth.rs#Session`

### `.noslop/` Directory

```
.noslop/
├── .gitkeep           # Ensure directory exists
├── staged-acks.json   # Pending acknowledgments
└── reviews/           # Review session JSON files
    └── REV-xxxx.json
```

## Dependencies

### Runtime Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `anyhow` | 1.0.100 | Error handling |
| `chrono` | 0.4.42 | Timestamps |
| `clap` | 4.5.51 | CLI parsing (derive) |
| `colored` | 3.0.0 | Terminal colors |
| `env_logger` | 0.11.8 | Logging |
| `git2` | 0.20.2 | Git operations (vendored OpenSSL) |
| `glob` | 0.3.3 | Glob pattern matching |
| `log` | 0.4.28 | Logging facade |
| `regex` | 1.12 | Pattern matching |
| `serde` | 1.0.228 | Serialization |
| `serde_json` | 1.0.145 | JSON handling |
| `thiserror` | 2.0.17 | Error types |
| `toml` | 0.9.8 | TOML parsing |
| `walkdir` | 2.5.0 | Directory traversal |

**Optional (feature-gated)**:
- `tokio` (1.48.0) + `reqwest` (0.12.24) for LLM features

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `assert_cmd` | CLI testing |
| `mockall` | Mock generation for port traits |
| `predicates` | Test assertions |
| `pretty_assertions` | Diff assertions |
| `proptest` | Property-based testing |
| `tempfile` | Temporary directories |
| `test-case` | Parameterized tests |

## Code Conventions

### Clippy Settings

From `lib.rs`:
```rust
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::cargo_common_metadata
)]
```

From `clippy.toml`:
- `cognitive-complexity-threshold = 30`
- `too-many-arguments-threshold = 7`
- `too-many-lines-threshold = 200`

### Error Handling

- All fallible functions return `anyhow::Result<T>`
- Custom errors use `thiserror::Error` derive
- No panics in library code (except documented `expect()` for invariants)

### Module Structure

- One file per model/port/adapter
- `mod.rs` files provide module documentation and re-exports
- Public types re-exported at crate root for convenience

## Test Patterns

### Test Organization

```
tests/
├── common/
│   ├── fixtures.rs   # CheckBuilder, AckBuilder
│   ├── git_repo.rs   # TempGitRepo helper
│   └── mocks.rs      # MockCheckRepository, MockAckStore, MockVersionControl
├── unit/             # Tests for core logic
├── adapter/          # Tests for adapter implementations
└── integration/      # End-to-end tests
```

### Test Fixtures

**`CheckBuilder`**:
```rust
let check = CheckBuilder::new()
    .id("TEST-1")
    .target("*.rs")
    .message("Test check")
    .severity(Severity::Block)
    .build();
```

**`TempGitRepo`**:
```rust
let repo = TempGitRepo::new();
repo.write_file("src/main.rs", "fn main() {}");
repo.stage("src/main.rs");
repo.commit("Initial commit");
```

### Mock Implementations

All port traits have manual mock implementations in `tests/common/mocks.rs`:
- `MockCheckRepository` - configurable check list
- `MockAckStore` - in-memory staging
- `MockVersionControl` - configurable staged files

Additionally, traits are annotated with `mockall::automock` for auto-generated mocks.

### Unit Tests

- Located in module files under `#[cfg(test)]`
- Use `test-case` for parameterized tests
- Use `proptest` for property-based testing (matcher tests)

### Integration Tests

- Full CLI invocation via `assert_cmd`
- Temporary git repos via `TempGitRepo`
- Mutex-protected directory changes for parallel test safety

## Existing Hook Installation System

The current init flow (`src/cli/commands/init.rs`):

1. Check if `.noslop.toml` exists (fail unless `--force`)
2. Generate project prefix from repo name
3. Create `.noslop.toml` with project config
4. Create `.noslop/` directory with `.gitkeep`
5. Install git hooks:
   - `install_pre_commit()` - validates checks
   - `install_commit_msg()` - adds trailers
   - `install_post_commit()` - clears staged acks
6. Check for existing docs (CLAUDE.md, AGENTS.md, ARCHITECTURE.md)

Hook files are:
- Created if missing
- Appended to if existing (preserves other content)
- Made executable on Unix

## Key Extension Points for Agent Integration

### Where Agent Config Would Fit

1. **New port trait**: `src/core/ports/agent.rs`
   - `AgentConfig` - configuration generation
   - `AgentRuntime` - invocation and output parsing

2. **New adapters**: `src/adapters/agent/`
   - `claude.rs` - Claude Code adapter
   - `codex.rs` - Codex adapter

3. **Extended init**: `src/cli/commands/init.rs`
   - `noslop init claude` / `noslop init codex`
   - Generate agent-specific config files

4. **New command**: `noslop run`
   - Iteration loop orchestrator
   - Uses `AgentRuntime` trait for agent invocation

5. **Config extension**: `.noslop.toml`
   ```toml
   [agent]
   type = "claude"
   # agent-specific settings

   [loop]
   max_iterations = 20
   # iteration loop settings
   ```

### Patterns to Follow

- Add new port trait in `src/core/ports/`
- Add adapter implementations in `src/adapters/`
- Add command in `src/cli/commands/`
- Add models in `src/core/models/`
- Use `anyhow::Result` for errors
- Annotate traits with `#[cfg_attr(test, mockall::automock)]`
- Write unit tests with fixtures and mocks
- Write integration tests with `TempGitRepo`

## Summary

Noslop has a clean hexagonal architecture that separates concerns well:

- **Core** is pure and testable
- **Ports** define clear interfaces
- **Adapters** handle all I/O
- **CLI** is thin dispatch layer

The codebase is well-suited for extension with agent support because:

1. The port/adapter pattern makes it easy to add new capabilities
2. Existing patterns (hooks, config, storage) can be reused
3. Test infrastructure supports mocking and integration testing
4. The init command already handles multi-step setup

Key areas that will need attention during migration:

1. Expanding `.noslop.toml` schema for agent config
2. Adding new files (progress.md, failures.jsonl) to `.noslop/`
3. Implementing agent invocation via trait
4. Translating bash scripts (ralph, checkpoint, worktree) to Rust
