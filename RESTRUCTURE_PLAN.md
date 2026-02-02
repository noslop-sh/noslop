# Codebase Restructure Plan

This document outlines the migration from the current flat structure to a hexagonal (ports & adapters) architecture, along with testing and quality improvements.

## Goals

1. **Separation of concerns**: Pure domain logic isolated from I/O
2. **Testability**: Core logic testable without filesystem/git
3. **Extensibility**: Easy to add Tauri GUI, LLM integration, new storage backends
4. **Maintainability**: Clear module boundaries as codebase grows

---

## Phase 1: Directory Structure Setup

Create the new directory skeleton without moving code yet.

- [ ] 1.1 Create `src/core/` directory structure
  - [ ] `src/core/mod.rs`
  - [ ] `src/core/models/mod.rs`
  - [ ] `src/core/services/mod.rs`
  - [ ] `src/core/ports/mod.rs`
- [ ] 1.2 Create `src/adapters/` directory structure
  - [ ] `src/adapters/mod.rs`
  - [ ] `src/adapters/toml/mod.rs`
  - [ ] `src/adapters/git/mod.rs`
  - [ ] `src/adapters/trailer/mod.rs`
  - [ ] `src/adapters/file/mod.rs`
- [ ] 1.3 Create `src/cli/` directory structure
  - [ ] `src/cli/mod.rs`
  - [ ] `src/cli/commands/mod.rs`
- [ ] 1.4 Create `src/shared/` directory structure
  - [ ] `src/shared/mod.rs`
  - [ ] `src/shared/parser/mod.rs`
- [ ] 1.5 Create new test directory structure
  - [ ] `tests/common/mod.rs`
  - [ ] `tests/common/fixtures.rs`
  - [ ] `tests/common/git_repo.rs`

---

## Phase 2: Define Port Traits

Define the interfaces before moving implementations.

- [ ] 2.1 Create `src/core/ports/assertion_repo.rs`
  - [ ] Define `AssertionRepository` trait
  - [ ] Methods: `find_for_files`, `add`, `remove`, `list`
- [ ] 2.2 Create `src/core/ports/attestation_store.rs`
  - [ ] Define `AttestationStore` trait (move from storage/mod.rs)
  - [ ] Methods: `stage`, `staged`, `clear_staged`, `format_trailers`
- [ ] 2.3 Create `src/core/ports/vcs.rs`
  - [ ] Define `VersionControl` trait
  - [ ] Methods: `staged_files`, `repo_name`, `install_hooks`
- [ ] 2.4 Export ports from `src/core/ports/mod.rs`

---

## Phase 3: Migrate Core Models

Move and refactor domain types.

- [ ] 3.1 Move `src/models/assertion.rs` → `src/core/models/assertion.rs`
  - [ ] Keep struct and impl blocks
  - [ ] Remove any I/O dependencies
- [ ] 3.2 Move `src/models/attestation.rs` → `src/core/models/attestation.rs`
- [ ] 3.3 Extract `Severity` enum to `src/core/models/severity.rs`
- [ ] 3.4 Move `src/target.rs` → `src/core/models/target.rs`
  - [ ] This is pure parsing logic, belongs in core
- [ ] 3.5 Create `src/core/models/mod.rs` with re-exports
- [ ] 3.6 Delete old `src/models/` directory
- [ ] 3.7 Update all imports throughout codebase
- [ ] 3.8 Run tests to verify no regressions

---

## Phase 4: Migrate Shared Utilities

Move cross-cutting code that doesn't fit domain or adapters.

- [ ] 4.1 Move `src/resolver.rs` → `src/shared/resolver.rs`
- [ ] 4.2 Move `src/parser/` → `src/shared/parser/`
  - [ ] `token.rs`
  - [ ] `pattern.rs`
  - [ ] `mod.rs`
- [ ] 4.3 Extract glob utilities from `noslop_file.rs` → `src/shared/glob.rs`
  - [ ] `matches_target` function (pure logic part)
  - [ ] Glob pattern compilation
- [ ] 4.4 Create `src/shared/mod.rs` with re-exports
- [ ] 4.5 Update imports and run tests

---

## Phase 5: Create Core Services

Extract pure business logic from commands and noslop_file.rs.

- [ ] 5.1 Create `src/core/services/matcher.rs`
  - [ ] Extract `matches_target` logic from `noslop_file.rs`
  - [ ] Make it a pure function: `(pattern, file, base_dir) -> bool`
  - [ ] No filesystem access - just string matching
- [ ] 5.2 Create `src/core/services/checker.rs`
  - [ ] Extract check logic from `commands/check.rs`
  - [ ] Generic over port traits: `CheckService<A, S, V>`
  - [ ] Pure orchestration: get files → match assertions → check attestations
- [ ] 5.3 Create `src/core/services/mod.rs`
- [ ] 5.4 Define `CheckResult` in core (not output.rs)
  - [ ] Move data structure, keep rendering in cli/output.rs
- [ ] 5.5 Write unit tests for matcher with mock data
- [ ] 5.6 Write unit tests for checker with mock ports

---

## Phase 6: Migrate Adapters

Move I/O implementations to adapters/.

- [ ] 6.1 Create `src/adapters/toml/parser.rs`
  - [ ] Move `NoslopFile`, `AssertionEntry`, `ProjectConfig` structs
  - [ ] Move `load_file`, `find_noslop_files` functions
- [ ] 6.2 Create `src/adapters/toml/writer.rs`
  - [ ] Move `add_assertion`, `format_noslop_file` functions
- [ ] 6.3 Create `src/adapters/toml/repository.rs`
  - [ ] Implement `AssertionRepository` trait
  - [ ] Compose parser + writer + matcher
- [ ] 6.4 Move `src/storage/trailer.rs` → `src/adapters/trailer/mod.rs`
  - [ ] Implement `AttestationStore` trait
- [ ] 6.5 Move `src/storage/file.rs` → `src/adapters/file/mod.rs`
  - [ ] Implement `AttestationStore` trait
- [ ] 6.6 Move `src/git/` → `src/adapters/git/`
  - [ ] `hooks.rs` - hook installation
  - [ ] `staging.rs` - staged file detection
  - [ ] `mod.rs` - implement `VersionControl` trait
- [ ] 6.7 Delete old `src/storage/`, `src/git/`, `src/noslop_file.rs`
- [ ] 6.8 Create `src/adapters/mod.rs` with re-exports
- [ ] 6.9 Update imports and run tests

---

## Phase 7: Migrate CLI Layer

Move CLI-specific code to cli/.

- [ ] 7.1 Move `src/cli.rs` → `src/cli/app.rs`
  - [ ] Keep Clap definitions
  - [ ] Keep `run()` function
- [ ] 7.2 Move `src/output.rs` → `src/cli/output.rs`
  - [ ] Keep `OutputMode`, rendering logic
  - [ ] Import `CheckResult` from core
- [ ] 7.3 Move `src/commands/` → `src/cli/commands/`
  - [ ] `init.rs` - thin wrapper, calls adapters
  - [ ] `check.rs` - thin wrapper, calls `CheckService`
  - [ ] `assert_cmd.rs` - thin wrapper
  - [ ] `attest.rs` - thin wrapper
  - [ ] `add_trailers.rs` → `hooks.rs`
  - [ ] `clear_staged.rs` → merge into `hooks.rs`
- [ ] 7.4 Refactor commands to be thin wrappers
  - [ ] Inject dependencies (adapters implementing ports)
  - [ ] Delegate to core services
  - [ ] Handle only CLI concerns (output formatting, exit codes)
- [ ] 7.5 Create `src/cli/mod.rs`
- [ ] 7.6 Update `src/main.rs` to use `cli::run()`
- [ ] 7.7 Delete old command files
- [ ] 7.8 Run all tests

---

## Phase 8: Update Library Exports

Clean up the public API in lib.rs.

- [ ] 8.1 Rewrite `src/lib.rs`
  - [ ] Export `core::models` (Assertion, Attestation, Severity, Target)
  - [ ] Export `core::services` (CheckService, CheckResult)
  - [ ] Export `core::ports` (traits for library users)
  - [ ] Export `adapters` (default implementations)
  - [ ] Export `shared` (utilities)
  - [ ] Do NOT export `cli` (binary-only)
- [ ] 8.2 Update documentation comments
- [ ] 8.3 Run `cargo doc` to verify API documentation

---

## Phase 9: Restructure Tests

Reorganize tests to match new architecture.

- [ ] 9.1 Create `tests/common/mod.rs`
  - [ ] Move shared helpers from `tests/unit/common/`
- [ ] 9.2 Create `tests/common/fixtures.rs`
  - [ ] Test data builders for Assertion, Attestation
  - [ ] Mock implementations of port traits
- [ ] 9.3 Create `tests/common/git_repo.rs`
  - [ ] `TempGitRepo` helper (extract from lifecycle_test.rs)
- [ ] 9.4 Reorganize unit tests
  - [ ] `tests/unit/mod.rs` - test runner
  - [ ] `tests/unit/matcher_test.rs` - core::services::matcher
  - [ ] `tests/unit/checker_test.rs` - core::services::checker
  - [ ] `tests/unit/target_test.rs` - core::models::target
  - [ ] `tests/unit/models_test.rs` - core::models::*
- [ ] 9.5 Create adapter tests
  - [ ] `tests/adapter/mod.rs`
  - [ ] `tests/adapter/toml_test.rs`
  - [ ] `tests/adapter/trailer_test.rs`
  - [ ] `tests/adapter/git_test.rs`
- [ ] 9.6 Keep integration tests
  - [ ] `tests/integration/mod.rs`
  - [ ] `tests/integration/lifecycle_test.rs` (update imports)
  - [ ] `tests/integration/cli_test.rs`
- [ ] 9.7 Delete old test files
- [ ] 9.8 Run full test suite

---

## Phase 10: Add Testing Infrastructure

Enhance testing capabilities.

- [ ] 10.1 Add new dev dependencies to Cargo.toml
  - [ ] `mockall = "0.13"` - mock generation
  - [ ] `proptest = "1.5"` - property-based testing
  - [ ] `test-case = "3.3"` - parameterized tests
  - [ ] `pretty_assertions = "1.4"` - better diffs
- [ ] 10.2 Generate mocks for port traits
  - [ ] `MockAssertionRepository`
  - [ ] `MockAttestationStore`
  - [ ] `MockVersionControl`
- [ ] 10.3 Add property tests for matcher
  - [ ] Glob pattern properties
  - [ ] Target parsing roundtrip
- [ ] 10.4 Add parameterized tests for edge cases
- [ ] 10.5 Update .tarpaulin.toml for new structure
- [ ] 10.6 Verify coverage meets thresholds

---

## Phase 11: Update CI/CD

Update workflows for new structure.

- [ ] 11.1 Update `.github/workflows/ci.yml`
  - [ ] Update test commands if paths changed
  - [ ] Add mutation testing job (optional)
- [ ] 11.2 Update Makefile targets
  - [ ] `make test-unit` - run only unit tests
  - [ ] `make test-adapter` - run adapter tests
  - [ ] `make test-integration` - run integration tests
  - [ ] `make test` - run all tests
- [ ] 11.3 Update coverage configuration
  - [ ] Include all test types
  - [ ] Exclude CLI from coverage (hard to test)
- [ ] 11.4 Run full CI pipeline locally

---

## Phase 12: Documentation & Cleanup

Final polish.

- [ ] 12.1 Update module-level documentation
  - [ ] `//!` comments in each mod.rs
  - [ ] Explain the architecture in core/mod.rs
- [ ] 12.2 Update CLAUDE.md with new structure
- [ ] 12.3 Remove dead code warnings (`#[allow(dead_code)]`)
- [ ] 12.4 Run `cargo clippy` and fix all warnings
- [ ] 12.5 Run `cargo fmt`
- [ ] 12.6 Final test run
- [ ] 12.7 Delete this plan file (or move to docs/)

---

## Final Directory Structure

```
src/
├── lib.rs                    # Public API
├── main.rs                   # Binary entry
│
├── core/                     # Pure domain (NO I/O)
│   ├── mod.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── assertion.rs
│   │   ├── attestation.rs
│   │   ├── severity.rs
│   │   └── target.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── matcher.rs
│   │   └── checker.rs
│   └── ports/
│       ├── mod.rs
│       ├── assertion_repo.rs
│       ├── attestation_store.rs
│       └── vcs.rs
│
├── adapters/                 # I/O implementations
│   ├── mod.rs
│   ├── toml/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   ├── writer.rs
│   │   └── repository.rs
│   ├── git/
│   │   ├── mod.rs
│   │   ├── hooks.rs
│   │   └── staging.rs
│   ├── trailer/
│   │   └── mod.rs
│   └── file/
│       └── mod.rs
│
├── cli/                      # CLI (binary only)
│   ├── mod.rs
│   ├── app.rs
│   ├── output.rs
│   └── commands/
│       ├── mod.rs
│       ├── init.rs
│       ├── check.rs
│       ├── assert_cmd.rs
│       ├── attest.rs
│       └── hooks.rs
│
└── shared/                   # Utilities
    ├── mod.rs
    ├── glob.rs
    ├── resolver.rs
    └── parser/
        ├── mod.rs
        ├── token.rs
        └── pattern.rs

tests/
├── common/
│   ├── mod.rs
│   ├── fixtures.rs
│   └── git_repo.rs
├── unit/
│   ├── mod.rs
│   ├── matcher_test.rs
│   ├── checker_test.rs
│   ├── target_test.rs
│   └── models_test.rs
├── adapter/
│   ├── mod.rs
│   ├── toml_test.rs
│   ├── trailer_test.rs
│   └── git_test.rs
└── integration/
    ├── mod.rs
    ├── lifecycle_test.rs
    └── cli_test.rs
```

---

## Risk Mitigation

1. **Incremental migration**: Each phase ends with passing tests
2. **No big bang**: Old code remains until new code is verified
3. **Git commits per phase**: Easy to bisect if issues arise
4. **Feature flags**: Can keep old paths temporarily if needed

---

## Success Criteria

- [ ] All existing tests pass
- [ ] `cargo clippy` produces no warnings
- [ ] `cargo doc` generates clean documentation
- [ ] Coverage remains above 80%
- [ ] Core services have 100% unit test coverage
- [ ] No I/O in `core/` module (enforced by no std::fs imports)
