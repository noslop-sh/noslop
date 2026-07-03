# Changelog

## v0.2.0 — 2026-07-03

noslop is now agent-first, evidence-backed, and CI-enforced. Highlights:

### Gate

- **Actor detection**: agents (Claude Code, Cursor, Codex, aider, CI) are
  gated by blocking checks; humans see the same guidance as an FYI and are
  never blocked. `NOSLOP_ACTOR` overrides detection.
- **Validated acknowledgments**: `noslop ack` requires an exact existing
  check ID; fuzzy matching removed.
- **Squash-proof ledger**: acks are versioned records committed into the
  tree (`.noslop/acks/`, compacted into `.noslop/history.jsonl` with
  union-merge). Trailers are now cosmetic; the tree is the record.
- `noslop init` always ensures hooks and per-clone state — safe to re-run
  on fresh clones of initialized repos.

### Discovery

- `noslop discover`: decompose CLAUDE.md / AGENTS.md / `.cursor/rules`
  into atomic, glob-scoped check proposals via your own agent CLI
  (`claude -p` auto-detected; `[discover] runner` to configure).
- `noslop discover --mine`: mine PR review history (via `gh`) for the
  conventions reviewers enforce by hand; `--from-file` mines exported
  JSONL dumps.
- Interactive review (`--review`): accept, edit, or reject each proposal;
  rejected rules are remembered — by key and by full text fed back into
  prompts — so they don't resurface, even paraphrased.

### Measurement

- Fire events with staged-tree fingerprints (`.noslop/events.jsonl`,
  per-clone) join with acks to distinguish self-correction from
  rubber-stamping.
- `noslop stats`: per-check fires, acks, self-corrections, stamps, dead
  targets (human / `--json` / `--markdown`).
- `noslop curate`: evidence-based prune/reword recommendations.

### CI

- `noslop check --ci --diff-base <ref>`: recompute checks against the
  branch diff and reconcile with committed ledger records — commits made
  with `--no-verify` carry no receipt and fail.
- GitHub Action (`uses: noslop-sh/noslop@main`): installs noslop, runs the
  diff-base gate, posts the payload to the job summary. Optional
  `upload-url`/`upload-token` POST the versioned envelope to an ingestion
  endpoint.

### Removed

- Deterministic markdown extraction heuristics (LLM-only discovery).
- Dead parser/resolver modules, compat shims, unused dependencies.

Data formats (ledger record, check payload, upload envelope) are
documented in [docs/SCHEMA.md](docs/SCHEMA.md) as schema 1.

## v0.1.2 — 2025-12-28

Initial public release: glob-scoped checks in `.noslop.toml`, pre-commit
gate, `noslop ack`, commit trailers.
