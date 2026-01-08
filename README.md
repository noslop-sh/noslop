# noslop

**Teach AI agents your codebase's unwritten rules.**

[![CI](https://github.com/noslop-sh/noslop/workflows/CI/badge.svg)](https://github.com/noslop-sh/noslop/actions)
[![Crates.io](https://img.shields.io/crates/v/noslop.svg)](https://crates.io/crates/noslop)

## The Problem

Your codebase has knowledge that lives in people's heads:

- "Migration files must be auto-generated, not hand-written"
- "New models need to be imported in `__init__.py` for Alembic to see them"
- "Public API routes need rate limiting"
- "Auth changes require security review"

AI agents don't know any of this. They write code that works, passes tests, and ignores everything your team actually cares about. You catch it in review, leave comments, wait for fixes, repeat.

**This is slop** - technically correct code that misses the point.

## The Solution

noslop creates a feedback loop that teaches agents your conventions at commit time. When an agent tries to commit, it receives contextual guidance - like a senior developer who never forgets to ask "did you remember X?"

```text
Agent commits → noslop intercepts → Agent gets guidance → Agent self-corrects
```

The agent fixes the issue, verifies that it addressed the guidance, and commits. No review round-trip. No repeating yourself.

## The Feedback Loop

Here's what makes noslop different from linters or pre-commit hooks:

1. **Agent writes code** that works but misses a convention
2. **Agent tries to commit** → noslop intercepts with relevant context
3. **Agent reads the guidance** ("Did you import in `__init__.py`?")
4. **Agent self-corrects** and verifies it addressed the issue
5. **You spot a new pattern** agents keep missing → add a check
6. **Your `.noslop.toml` grows** into institutional memory

Over time, you're not just blocking bad commits - you're building a context layer that makes agents smarter about *your* codebase.

## Quick Start

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/noslop-sh/noslop/main/scripts/install.sh | bash
# Or: cargo install noslop

# Initialize in your repo
cd your-project
noslop init
```

## Defining Checks

Add checks for patterns that trip up agents:

```toml
# .noslop.toml

[[check]]
target = "migrations/versions/*.py"
message = "Generated with alembic revision --autogenerate?"
severity = "block"

[[check]]
target = "models/**/*.py"
message = "Imported in models/__init__.py?"
severity = "block"

[[check]]
target = "api/public/*_router.py"
message = "Rate limiting decorator added?"
severity = "block"
```

When an agent commits changes to these paths:

```text
$ git commit -m "Add User model"

BLOCKED: 2 unverified checks

  [DB-1] models/user.py
         Imported in models/__init__.py?

  [DB-2] migrations/versions/abc123_add_user.py
         Generated with alembic revision --autogenerate?

To proceed: noslop verify <id> -m "verification"
```

The agent sees exactly what it missed, fixes it, and verifies:

```bash
noslop verify DB-1 -m "Added import to models/__init__.py"
noslop verify DB-2 -m "Regenerated with alembic --autogenerate"
git commit -m "Add User model"  # Now succeeds
```

## Verifications as Learning Records

Verifications aren't just checkboxes - they're a record of what the agent acknowledged and how it addressed the guidance. They're stored as git trailers, visible in your commit history:

```text
Add User model

Noslop-Verify: DB-1 | Added import to models/__init__.py | claude-3-opus
Noslop-Verify: DB-2 | Regenerated with alembic --autogenerate | claude-3-opus
```

## Building Your Context Layer

Every time you catch an agent missing something in review, ask yourself: *"Could I have told the agent this at commit time?"*

If yes, add a check:

```bash
noslop check add "config/prod.yaml" \
  -m "Production config changes need #ops-review approval" \
  --severity block
```

Your `.noslop.toml` becomes a living document - institutional knowledge that compounds over time:

```text
Week 1: Agent forgets __init__.py imports     → Add check
Week 2: Agent forgets rate limiting           → Add check
Week 3: Agent hand-writes migrations          → Add check
Week 4: Agent handles all three automatically
```

The more you use it, the less you repeat yourself.

## Target Patterns

```toml
target = "migrations/versions/*.py"     # Files in directory
target = "models/**/*.py"               # Recursive glob
target = "*_router.py"                  # Suffix match anywhere
target = "config/prod.yaml"             # Specific file
```

## Severity Levels

- **block** - Commit blocked until verified (for hard requirements)
- **warn** - Warning shown, commit proceeds (for soft guidelines)
- **info** - Informational context (for helpful reminders)

## Commands

```bash
noslop init                              # Set up in repo
noslop check add <target> -m <message>   # Add check
noslop check list                        # List all checks
noslop check remove <id>                 # Remove check
noslop verify <id> -m <message>          # Verify a check
noslop check run                         # Check staged files
```

## Why "noslop"?

Slop is code that technically works but misses everything that matters. It compiles, passes tests, and ignores every convention your team has built up over years.

noslop is the feedback loop that fixes this - teaching agents your codebase's unwritten rules so they stop producing slop and start producing code that actually fits.

## License

See [LICENSE](LICENSE).

## Links

- [Website](https://noslop.sh)
- [GitHub](https://github.com/noslop-sh/noslop)
- [Issues](https://github.com/noslop-sh/noslop/issues)
