# noslop v0.2.0 Roadmap

Teach AI agents (and humans) codebase conventions through automated feedback at commit time.

> **Living Document**: This checklist is updated as features are completed. Each feature is developed on its own branch, tested, and merged sequentially.

---

## Completed

- [x] Hexagonal architecture migration
- [x] Terminology: `assertion/attest` → `check/ack`
- [x] Hot-reload dev workflow (`make dev`)

---

## In Progress

Current feature branch: `fix/force-hook-reinstall`

---

## v0.2.0 Backlog

Features are worked sequentially. Each item becomes a branch (`feature/<name>`), gets tests, and is merged before starting the next.

### 1. Bug Fixes

- [ ] **fix/force-hook-reinstall** - Fix `--force` hook reinstall ([mod.rs:74](src/adapters/git/mod.rs#L74))
- [ ] **fix/glob-applies-to** - Glob pattern support in `applies_to()` ([check.rs:50](src/core/models/check.rs#L50))

### 2. Check Engine

- [ ] **feature/conditional-checks** - Logic expressions (`when: "branch != 'main'"`) or prompt-based evaluation
- [ ] **feature/ast-parsing** - Generic grammar support (tree-sitter) with SLM fallback
- [ ] **feature/semantic-targets** - Target by symbol (`src/auth.rs#fn:login`)

### 3. Core CLI

- [ ] **feature/json-output** - JSON output (`--format=json`) for CI/agent consumption
- [ ] **feature/doctor** - `noslop doctor` command to diagnose config and hook issues
- [ ] **feature/bulk-ack** - Bulk ack (`noslop ack --all`)

### 4. Integration

- [ ] **feature/github-action** - GitHub Actions action (`noslop-sh/noslop-action@v1`)

### 5. Documentation

- [ ] **docs/getting-started** - Getting started guide
- [ ] **docs/agent-integration** - AI agent integration guide (Claude, Copilot, Cursor)

---

## Future (post v0.2.0)

- [ ] VS Code extension
- [ ] MCP server
- [ ] GitHub PR comments
