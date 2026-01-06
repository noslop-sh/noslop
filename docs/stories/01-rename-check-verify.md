# Story 01: Rename to Check/Verify

**Status: COMPLETE**

## Summary

Renamed core concepts for clarity:
- `check` - what gets verified at commit time
- `verify` / `verification` - acknowledging a check was addressed

## Motivation

The previous naming was confusing:
- "Assertion" sounded like a test assertion (x == y)
- "Attestation" was uncommon jargon
- Users had to learn noslop-specific vocabulary

The new naming is intuitive:
- "Check" is what happens at commit time
- "Verify" is what you do to pass a check

## User Stories

### US-01.1: Define checks in TOML
**As a** developer
**I want to** define checks in `.noslop.toml`
**So that** my rules are enforced at commit time

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

```bash
noslop check add "migrations/*.py" -m "Generated with alembic?"
noslop check list
noslop check remove NOS-1
```

### US-01.3: Run checks on staged files
**As a** developer
**I want to** run checks against staged files
**So that** I can see what needs verification before committing

```bash
noslop check run
```

### US-01.4: Verify a check
**As a** developer
**I want to** verify that I've addressed a check
**So that** my commit can proceed

```bash
noslop verify NOS-1 -m "Ran alembic --autogenerate"
```

### US-01.5: See verifications in commit trailers
**As a** developer
**I want to** see verification records in commit history
**So that** there's an audit trail

```
Add user migration

Noslop-Verify: NOS-1 | Ran alembic --autogenerate | human
```

## Technical Changes

### File Renames
| Old | New |
|-----|-----|
| `src/models/assertion.rs` | `src/models/check.rs` |
| `src/models/attestation.rs` | `src/models/verification.rs` |
| `src/commands/assert_cmd.rs` | `src/commands/check_cmd.rs` |
| `src/commands/attest.rs` | `src/commands/verify.rs` |

### Struct Renames
| Old | New |
|-----|-----|
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
Pre-commit hook invokes:
```bash
noslop check run
```

### TOML Parsing
- Parses `[[check]]` sections
- No backward compatibility (clean break)

## Acceptance Criteria

- [x] `noslop check add/list/remove` works
- [x] `noslop check run` runs pre-commit checks
- [x] `noslop verify <id>` records verification
- [x] `[[check]]` parsed from TOML
- [x] `Noslop-Verify:` trailer in commits
- [x] All tests pass with new naming
- [x] README updated
- [x] No legacy terminology in codebase
