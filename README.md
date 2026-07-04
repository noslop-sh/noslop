# noslop

**Teach AI agents your codebase's unwritten rules — and get receipts.**

[![CI](https://github.com/noslop-sh/noslop/workflows/CI/badge.svg)](https://github.com/noslop-sh/noslop/actions)
[![Crates.io](https://img.shields.io/crates/v/noslop.svg)](https://crates.io/crates/noslop)

## The Problem

Your codebase has knowledge that lives in people's heads:

- "Migration files must be auto-generated, not hand-written"
- "New models need to be imported in `__init__.py` for Alembic to see them"
- "Public API routes need rate limiting"

You wrote it all down in CLAUDE.md or AGENTS.md — and agents still miss it.
Rules files are advisory: models follow a fraction of a long instruction
list, and nothing records whether a rule was ever considered. You catch
violations in review, leave the same comments again, and repeat.

**This is slop** — technically correct code that misses the point.

## The Solution

noslop turns your conventions into small, glob-scoped **checks** that fire
at commit time, exactly when the files they guard are being changed:

```text
Agent commits → matching checks block → agent fixes & acknowledges → commit proceeds
```

Three properties make it different from a linter or a rules file:

1. **It gates agents, never humans.** noslop detects who is committing
   (Claude Code, Cursor, Codex, aider, CI — or a person). Agents must
   acknowledge blocking checks; humans see the same guidance as an FYI and
   are never blocked. Hotfixes stay fast.
2. **Acknowledgments are receipts, not checkboxes.** `noslop ack` requires
   the exact check ID and records who acknowledged, what they claimed, and
   a fingerprint of the staged tree — written into the repository itself,
   so the audit trail survives squash merges and rebases.
3. **The rulebook is measured, not static.** Every check firing and every
   ack is recorded. `noslop stats` shows which checks cause real
   action rate and which get no-action; `noslop curate` tells you
   what to prune or reword, with evidence.

## Quick Start

```bash
# Install
cargo install noslop --locked
# Or download the install script, review it, then run it. It verifies the
# release sha256 before installing and refuses to install otherwise.
#   curl -fsSLO https://raw.githubusercontent.com/noslop-sh/noslop/main/scripts/install.sh
#   less install.sh && bash install.sh

cd your-project
noslop init        # hooks + config; safe to re-run on fresh clones
noslop discover    # propose checks from CLAUDE.md / AGENTS.md / .cursor/rules
noslop discover --review   # accept, edit, or reject each proposal
```

`discover` decomposes your existing rules files into atomic checks using
whatever agent CLI you already have installed (`claude -p` by default;
configurable). Nothing enforces until you accept it.

Have review history? Mine the conventions your team enforces by hand:

```bash
noslop discover --mine                    # last ~600 PR review comments via gh
noslop discover --from-file dump.jsonl    # or from an exported dump
```

## Defining Checks

```toml
# .noslop.toml

[[check]]
id = "API-1"
target = "api/public/*_router.py"
message = "Rate limiting decorator added?"
severity = "block"
```

When an agent commits changes to matching paths:

```text
$ git commit -m "Add users endpoint"

Blocking:
  [API-1] api/public/users_router.py
          Rate limiting decorator added?

BLOCKED: 1 unacknowledged check(s)
```

The agent fixes the issue and acknowledges with its identity:

```bash
noslop ack API-1 -m "Added @rate_limit(100/min) to all new routes"
git commit -m "Add users endpoint"   # proceeds
```

The receipt is recorded as a ledger file committed with the change, plus a
cosmetic trailer:

```text
Noslop-Ack: API-1 | Added @rate_limit(100/min) to all new routes | claude-code
```

## CI as the Source of Truth

Local hooks can be skipped (`--no-verify`); the ledger cannot. The GitHub
Action recomputes checks against the pull request diff and reconciles them
with the ledger records committed in the branch — a commit that skipped the
gate carries no receipt and fails CI:

```yaml
# .github/workflows/noslop.yml
on:
  pull_request:
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: dtolnay/rust-toolchain@stable
      - uses: noslop-sh/noslop@main
```

## Measuring the Rulebook

```bash
noslop stats     # per check: fires, acks, action rates vs no-action answers, dead targets
noslop curate    # what to prune or reword, with evidence
```

An acknowledgment that changed nothing between fire and answer took no
action; one where files changed means the guidance worked. Checks whose
targets no longer exist are flagged dead. Your `.noslop.toml` stays a
living document instead of rotting like a rules file.

## Actor Detection

| Committer                         | Detected as | Blocking checks             |
| --------------------------------- | ----------- | --------------------------- |
| Claude Code, Cursor, Codex, aider | agent name  | must be acknowledged        |
| CI (`--ci` or CI env)             | `ci`        | enforced                    |
| A person at a terminal            | `human`     | shown as FYI, never blocked |

Override with `NOSLOP_ACTOR=<name>` when needed.

## Commands

```bash
noslop init                              # Set up in repo (safe on fresh clones)
noslop discover                          # Propose checks from rules files (uses your agent CLI)
noslop discover --mine                   # Mine checks from PR review history (gh)
noslop discover --from-file <jsonl>      # Mine from an exported comment dump
noslop discover --review                 # Accept, edit, or reject proposals
noslop check                             # Validate staged files (pre-commit hook)
noslop check --ci --diff-base <ref>      # CI: validate branch diff against the ledger
noslop check add <target> -m <message>   # Add a check by hand
noslop check list                        # List all checks
noslop check remove <id>                 # Remove a check
noslop ack <id> -m <message>             # Acknowledge a check (exact ID required)
noslop stats [--markdown]                # Per-check metrics
noslop curate [--markdown]               # Prune/reword recommendations
noslop compact                           # Fold ack records into history (run at merge)
```

## Severity Levels

- **block** — agents must acknowledge before committing
- **warn** — shown prominently, never blocks
- **info** — contextual reminder

## Data Formats

The ack ledger, check payload, and upload envelope are versioned formats —
see [docs/SCHEMA.md](docs/SCHEMA.md). This repo dogfoods noslop: its own
commits are gated by the checks in [.noslop.toml](.noslop.toml), and the
receipts are in [.noslop/](.noslop/).

## License

AGPL-3.0 — see [LICENSE](LICENSE).

## Links

- [Website](https://noslop.sh)
- [GitHub](https://github.com/noslop-sh/noslop)
- [Issues](https://github.com/noslop-sh/noslop/issues)
