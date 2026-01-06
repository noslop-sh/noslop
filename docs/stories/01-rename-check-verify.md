# Story 01: Rename Assert/Attest to Check/Verify

## Summary

Rename core concepts for clarity:
- `assert` / `assertion` → `check`
- `attest` / `attestation` → `verify` / `verification`

## Motivation

The current naming is confusing:
- "Assertion" sounds like a test assertion (assert x == y)
- "Attestation" is uncommon jargon
- Users have to learn noslop-specific vocabulary

The new naming is intuitive:
- "Check" is what happens at commit time
- "Verify" is what you do to pass a check

## User Stories

### US-01.1: Define checks in TOML
**As a** developer
**I want to** define checks in `.noslop.toml`
**So that** my rules are enforced at commit time

**Before:**
```toml
[[assert]]
id = "NOS-1"
target = "migrations/*.py"
message = "Generated with alembic?"
severity = "block"
```

**After:**
```toml
[[check]]
id = "NOS-1"
target = "migrations/*.py"
message = "Generated with alembic?"
severity = "block"
```

### US-01.2: Manage checks via CLI
**As a** developer
**I want to** add, list, and remove checks via CLI
**So that** I can manage rules without editing TOML

**Before:**
```bash
noslop assert add "migrations/*.py" -m "Generated with alembic?"
noslop assert list
noslop assert remove NOS-1
```

**After:**
```bash
noslop check add "migrations/*.py" -m "Generated with alembic?"
noslop check list
noslop check remove NOS-1
```

### US-01.3: Run checks on staged files
**As a** developer
**I want to** run checks against staged files
**So that** I can see what needs verification before committing

**Before:**
```bash
noslop check
```

**After:**
```bash
noslop check run
```

### US-01.4: Verify a check
**As a** developer
**I want to** verify that I've addressed a check
**So that** my commit can proceed

**Before:**
```bash
noslop attest NOS-1 -m "Ran alembic --autogenerate"
```

**After:**
```bash
noslop verify NOS-1 -m "Ran alembic --autogenerate"
```

### US-01.5: See verifications in commit trailers
**As a** developer
**I want to** see verification records in commit history
**So that** there's an audit trail

**Before:**
```
Add user migration

Noslop-Attest: NOS-1 | Ran alembic --autogenerate | human
```

**After:**
```
Add user migration

Noslop-Verify: NOS-1 | Ran alembic --autogenerate | human
```

## Technical Changes

### File Renames
| Current | New |
|---------|-----|
| `src/models/assertion.rs` | `src/models/check.rs` |
| `src/models/attestation.rs` | `src/models/verification.rs` |
| `src/commands/assert_cmd.rs` | `src/commands/check_cmd.rs` |
| `src/commands/attest.rs` | `src/commands/verify.rs` |

### Struct Renames
| Current | New |
|---------|-----|
| `Assertion` | `Check` |
| `Attestation` | `Verification` |
| `AssertAction` | `CheckAction` |
| `AssertionMatch` | `CheckMatch` |

### CLI Structure
```
noslop
├── check
│   ├── run          # Run checks (pre-commit hook)
│   ├── add          # Add a check
│   ├── list         # List checks
│   └── remove       # Remove a check
├── verify <id>      # Verify a check
├── task             # (unchanged)
├── status           # (unchanged)
└── init             # (unchanged)
```

### Git Hooks
Pre-commit hook content changes:
```bash
# Before
noslop check

# After
noslop check run
```

### TOML Parsing
- Parse `[[check]]` instead of `[[assert]]`
- Maintain backward compatibility: warn if `[[assert]]` found, suggest migration

## Acceptance Criteria

- [ ] `noslop check add/list/remove` works
- [ ] `noslop check run` runs pre-commit checks
- [ ] `noslop verify <id>` records verification
- [ ] `[[check]]` parsed from TOML
- [ ] `Noslop-Verify:` trailer in commits
- [ ] All tests pass with new naming
- [ ] README updated
- [ ] Backward compat warning for `[[assert]]`

## Migration Path

1. `noslop init` creates new format
2. Old format `[[assert]]` still works but shows deprecation warning
3. Add `noslop migrate` command to convert old format (optional)
