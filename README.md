# noslop

**Context-aware reminders for AI coding agents.** Stop AI from breaking conventions it doesn't know exist.

[![CI](https://github.com/noslop-sh/noslop/workflows/CI/badge.svg)](https://github.com/noslop-sh/noslop/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/noslop-sh/noslop/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/noslop.svg)](https://crates.io/crates/noslop)

## What is noslop?

AI coding agents are powerful but context-blind. They don't know your undocumented conventions:

- Migration files must be generated with `alembic revision --autogenerate`, not hand-written
- New models need to be imported in `models/__init__.py` for Alembic to detect them
- Public API routers need rate limiting decorators
- Auth directory changes require security review tickets

**noslop makes AI agents aware of these requirements before they commit.** When an agent attempts to commit code, it sees relevant assertions and can self-correct, catching issues before review instead of during it.

## How It Works

When an AI agent (or human) commits code, noslop's pre-commit hook shows assertions for matching files. The agent sees the requirement, adjusts its approach, attests to the fix, and commits successfully, all before pushing to review.

```toml
# .noslop.toml at repository root
[[assert]]
target = "migrations/versions/*.py"
message = "Generated with alembic revision --autogenerate?"
severity = "block"

[[assert]]
target = "models/**/*.py"
message = "New models imported in models/__init__.py for Alembic detection?"
severity = "block"

[[assert]]
target = "api/public/*_router.py"
message = "Added rate limiting decorator?"
severity = "block"
```

When committing, the agent sees:

```bash
$ git commit -m "Add User model and migration"

BLOCKED: 2 unattested assertions

  [DB-1] models/user.py
          New models imported in models/__init__.py for Alembic detection?

  [DB-2] migrations/versions/abc123_add_user.py
          Generated with alembic revision --autogenerate?

To proceed: noslop attest <id> -m "verification"
```

The agent can now:

1. Check if it forgot to import the model
2. Verify the migration was auto-generated
3. Attest and commit with the fixes

## Installation

```bash
# macOS/Linux
curl -fsSL https://raw.githubusercontent.com/noslop-sh/noslop/main/scripts/install.sh | bash

# From crates.io
cargo install noslop

# From source
git clone https://github.com/noslop-sh/noslop.git
cd noslop
cargo install --path .
```

## Usage

### Initialize

```bash
cd your-project
noslop init
```

Creates `.noslop.toml` and installs git hooks.

### Define assertions

```bash
noslop assert add "migrations/versions/*.py" \
  -m "Generated with alembic revision --autogenerate?" \
  --severity block

noslop assert add "models/**/*.py" \
  -m "New models imported in models/__init__.py for Alembic detection?" \
  --severity block

noslop assert add "api/public/*_router.py" \
  -m "Added rate limiting decorator?" \
  --severity block
```

### Commit workflow (for agents)

1. Agent modifies `models/user.py` and creates `migrations/versions/abc_add_user.py`
2. Agent runs `git commit -m "Add User model"`
3. Noslop shows 2 blocking assertions
4. Agent sees it forgot to import User in `models/__init__.py`
5. Agent adds import, attests: `noslop attest DB-1 -m "Added to models/__init__.py"`
6. Agent verifies migration was auto-generated: `noslop attest DB-2 -m "Generated with alembic"`
7. Agent commits successfully

Attestations are recorded in commit history as git trailers:

```text
Add User model

Noslop-Attest: DB-1 | Added to models/__init__.py | agent
Noslop-Attest: DB-2 | Generated with alembic | agent
```

## Examples

### Alembic Model Registration

```toml
# Catch forgotten model imports
[[assert]]
target = "models/**/*.py"
message = "New models imported in models/__init__.py for Alembic detection?"
severity = "block"
tags = ["database", "alembic"]

# Verify migrations are auto-generated
[[assert]]
target = "migrations/versions/*.py"
message = "Generated with alembic revision --autogenerate?"
severity = "block"
tags = ["database", "alembic"]
```

Common issue: Agent creates a new SQLAlchemy model but forgets to import it in `models/__init__.py`, causing Alembic to not detect it during `alembic revision --autogenerate`. Noslop reminds the agent to check the import.

### Public API Rate Limiting

```toml
[[assert]]
target = "api/public/*_router.py"
message = "Added rate limiting decorator?"
severity = "block"
tags = ["api", "security"]
```

Applies to router files in the public API directory. Agent sees this before committing and adds `@rate_limit(100)`.

### Security Reviews

```toml
[[assert]]
target = "auth/**/*.py"
message = "Security review ticket number?"
severity = "block"
tags = ["security"]
```

Any Python file in the auth directory requires security team signoff. Agent can note the ticket in attestation.

### Team Conventions

```toml
[[assert]]
target = "components/**/*.tsx"
message = "Uses useQuery hook instead of raw fetch?"
severity = "warn"
tags = ["react", "patterns"]
```

Warns agents about data-fetching patterns without blocking the commit.

## Target Patterns

```toml
# Directory patterns
target = "migrations/versions/*.py"     # All files in directory
target = "auth/**/*.py"                 # Recursive match
target = "models/**/*.py"               # All model files

# File naming conventions
target = "*_router.py"                  # All router files
target = "*_handler.py"                 # All handler files
target = "test_*.py"                    # All test files

# Specific files
target = "config/prod.yaml"             # Production config only
target = "models/__init__.py"           # Model registry

# Multiple extensions
target = "*.{env,env.*}"                # Environment files
```

**Best practice**: Target patterns that represent repeat considerations (all routers, all migrations), not one-time tasks.

## Severity Levels

- **`block`**: Commit blocked until attested (critical requirements)
- **`warn`**: Warning shown but commit proceeds (best practices)
- **`info`**: Informational only (helpful reminders)

## Commands

```bash
noslop init                              # Initialize in repository
noslop assert add <target> -m <message>  # Add assertion
noslop assert list                       # List all assertions
noslop assert remove <id>                # Remove assertion
noslop attest <id> -m <message>          # Attest to assertion
noslop check                             # Check staged files (runs in pre-commit)
```

## For AI Agents

Noslop integrates with AI coding workflows:

1. **Agent makes changes** - Adds files, modifies code
2. **Agent commits** - Runs `git commit`
3. **Noslop shows assertions** - Agent sees requirements it may have missed
4. **Agent self-corrects** - Fixes issues, adds missing imports, verifies conventions
5. **Agent attests** - Documents what it checked
6. **Commit succeeds** - With full context in git history

This catches common mistakes like forgotten imports, missing decorators, and wrong patterns **before code review**, reducing iteration cycles.

## Advanced: Multiple Config Files

You can place `.noslop.toml` files in subdirectories for scoped assertions. Files inherit from parent configs.

## Why "noslop"?

Slop is well-intentioned but low-quality output. Code that compiles but violates conventions. noslop helps AI (and humans) maintain quality by surfacing context at the right time.

## License

MIT License. See [LICENSE](LICENSE) for details.

## Links

- [Documentation](https://github.com/noslop-sh/noslop/tree/main/docs)
- [Issue Tracker](https://github.com/noslop-sh/noslop/issues)
- [Changelog](https://github.com/noslop-sh/noslop/releases)
