# noslop

Agent-first commit gate: breaks team conventions into small, glob-scoped checks that agents must acknowledge at commit time. Acks are recorded for audit. Humans are never hard-blocked (design invariant).

## Commands

- `make check` — fmt + clippy (pedantic, deny warnings) + full test suite. Must pass before every commit.
- `make test-unit | test-adapter | test-integration | test-lib` — targeted test runs.
- `cargo install --path .` — install the local binary (hooks call `noslop` from PATH).

## Architecture

- `src/core/` — pure domain logic, no I/O: models (Check, Acknowledgment, Severity, Target), ports (traits), services (matcher, checker).
- `src/adapters/` — I/O implementations: toml (check files), git (hooks, staging), trailer (commit trailers), file (staged acks in `.noslop/`).
- `src/cli/` — clap app + command handlers. CLI must call core services, never reimplement their logic.
- Flow: pre-commit hook → `noslop check` matches staged files against `.noslop.toml` checks → block until `noslop ack` → commit-msg hook appends `Noslop-Ack:` trailers → post-commit clears staged acks.

## Conventions

- Branches: `sp/no-<linear-issue>-<slug>`. Never commit to main. PRs attach to their Linear issue (team "No Slop", prefix NO-).
- No dead code: `#![allow(dead_code)]` is banned; delete unused code instead of parking it.
- No compat shims or speculative abstractions: one implementation per port until a second consumer exists.
- New logic requires tests. Behavior changes require updating the affected tests in the same commit.
- This repo dogfoods noslop: hooks are active. Ack checks honestly after actually verifying them. Never use `--no-verify`.
- Versioned schemas (ack ledger, check-run payloads) are the stable API boundary — breaking them requires a version bump and migration note.
