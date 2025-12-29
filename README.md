# noslop

**The missing context layer for coding agents.**

[![CI](https://github.com/noslop-sh/noslop/workflows/CI/badge.svg)](https://github.com/noslop-sh/noslop/actions)
[![Crates.io](https://img.shields.io/crates/v/noslop.svg)](https://crates.io/crates/noslop)

## The Problem

Your team has knowledge that lives in people's heads:

- "Migration files must be auto-generated, not hand-written"
- "New models need to be imported in `__init__.py` for Alembic"
- "Public API routes need rate limiting"
- "Auth changes require security review"

Agents don't know any of this. They'll write working code that misses every detail you care about. You catch it in review, leave comments, wait for fixes, repeat. Slop.

## The Solution

noslop surfaces what agents need to know, when they need to know it. They see what they missed and fix it themselves.

```toml
# .noslop.toml
[[assert]]
target = "migrations/versions/*.py"
message = "Generated with alembic revision --autogenerate?"
severity = "block"

[[assert]]
target = "models/**/*.py"
message = "Imported in models/__init__.py?"
severity = "block"
```

When an agent commits:

```
$ git commit -m "Add User model"

BLOCKED: 2 unattested assertions

  [DB-1] models/user.py
         Imported in models/__init__.py?

  [DB-2] migrations/versions/abc123_add_user.py
         Generated with alembic revision --autogenerate?

To proceed: noslop attest <id> -m "verification"
```

The agent sees the problem, fixes it, attests, and commits. No review round-trip.

## Installation

```bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/noslop-sh/noslop/main/scripts/install.sh | bash

# Or via cargo
cargo install noslop
```

## Quick Start

```bash
cd your-project
noslop init
```

This creates `.noslop.toml` and installs git hooks.

## Defining Assertions

Add assertions for patterns that trip up agents repeatedly:

```bash
# Database conventions
noslop assert add "migrations/versions/*.py" \
  -m "Generated with alembic revision --autogenerate?" \
  --severity block

noslop assert add "models/**/*.py" \
  -m "Imported in models/__init__.py?" \
  --severity block

# API conventions
noslop assert add "api/public/*_router.py" \
  -m "Rate limiting decorator added?" \
  --severity block

# Security requirements
noslop assert add "auth/**/*.py" \
  -m "Security review ticket?" \
  --severity block
```

## How Agents Use It

1. Agent modifies code
2. Agent runs `git commit`
3. noslop shows what they need to know
4. Agent self-corrects
5. Agent attests: `noslop attest DB-1 -m "Added to __init__.py"`
6. Commit succeeds

Attestations are recorded as git trailers, visible in your commit history:

```
Add User model

Noslop-Attest: DB-1 | Added to __init__.py | claude-3-opus
Noslop-Attest: DB-2 | Generated with alembic | claude-3-opus
```

## Real Examples

### Alembic + SQLAlchemy

```toml
[[assert]]
target = "models/**/*.py"
message = "Imported in models/__init__.py?"
severity = "block"
tags = ["database"]

[[assert]]
target = "migrations/versions/*.py"
message = "Generated with alembic revision --autogenerate?"
severity = "block"
tags = ["database"]
```

Catches the classic mistake: agent creates a model, writes a migration by hand, but forgets the import that makes Alembic see the model.

### API Security

```toml
[[assert]]
target = "api/public/*_router.py"
message = "Rate limiting decorator added?"
severity = "block"
tags = ["security"]
```

### Team Patterns

```toml
[[assert]]
target = "components/**/*.tsx"
message = "Using useQuery instead of raw fetch?"
severity = "warn"
tags = ["react"]
```

`warn` shows the convention but doesn't block. Good for best practices vs hard requirements.

## Target Patterns

```toml
target = "migrations/versions/*.py"     # Files in directory
target = "auth/**/*.py"                 # Recursive
target = "*_router.py"                  # Suffix match
target = "config/prod.yaml"             # Specific file
```

## Severity Levels

- **block** - Commit blocked until attested
- **warn** - Warning shown, commit proceeds
- **info** - Informational only

## Commands

```bash
noslop init                              # Set up in repo
noslop assert add <target> -m <message>  # Add convention
noslop assert list                       # List all
noslop assert remove <id>                # Remove
noslop attest <id> -m <message>          # Attest
noslop check                             # Check staged files
```

## Why "noslop"?

Slop is code that works but misses the point. It compiles, passes tests, and ignores everything your team knows. noslop is the fix.

## License

See [LICENSE](LICENSE).

## Links

- [Website](https://noslop.sh)
- [Issues](https://github.com/noslop-sh/noslop/issues)
- [Releases](https://github.com/noslop-sh/noslop/releases)
