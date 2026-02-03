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

Current feature branch: none

---

## v0.2.0 Backlog

Features are worked sequentially. Each item becomes a branch (`feature/<name>`), gets tests, and is merged before starting the next.

### 1. Bug Fixes

- [x] **fix/force-hook-reinstall** - Fix `--force` hook reinstall ✓
- [x] **fix/glob-applies-to** - Glob pattern support in `applies_to()` ✓

### 2. Check Engine

- [ ] **feature/smart-checks** - Prompt-based evaluation (`prompt: "Does this affect auth?"`) and conditional logic (`when: "branch != 'main'"`)

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

## Future (v0.3.0+)

- [ ] **Codebase indexing** - AST-based understanding (tree-sitter) to feed context into prompts
- [ ] VS Code extension
- [ ] MCP server
- [ ] GitHub PR comments
