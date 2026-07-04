# noslop data formats (schema v1)

These formats are the stable API boundary. Anything else (CLI output text,
internal file layout) may change freely; these may not change without a
`schema` version bump and a migration note in this file.

## Ack ledger record — `.noslop/acks/<check-id>-<digest>.json`

Written by `noslop ack`, staged into the same commit as the acknowledged
change. File content survives squash merges and rebases (trailers do not).

```json
{
  "schema": 1,
  "check_id": "NOS-2",
  "message": "exact-ID matching only; added tests",
  "acknowledged_by": "claude-code",
  "created_at": "2026-07-02T00:53:11.000Z",
  "tree_oid": "8f2c4b1e0a9d7c6b5a4f3e2d1c0b9a8f7e6d5c4b"
}
```

- `acknowledged_by` is the detected actor: `human`, an agent name
  (`claude-code`, `cursor`, `codex`, `aider`, `github-actions`, `ci`,
  `unknown-agent`), or the explicit `NOSLOP_ACTOR` value.
- `tree_oid` (optional, added within schema 1 as an additive field): the
  staged-tree object id (`git write-tree`) at ack time. Joined with fire
  events to distinguish self-correction from rubber-stamping. Records
  written before this field simply omit it.
- `fire_tree_oid`, `fired_at` (optional, added within schema 1 as additive
  fields): the staged-tree oid and timestamp of the check's latest local
  fire event, copied into the record at ack time so it is self-contained
  evidence — `fire_tree_oid == tree_oid` means the ack changed nothing
  (rubber stamp), and `fired_at → created_at` is the time-to-ack. Absent
  when no local fire event preceded the ack (e.g. ack before first check
  run) and in records written before these fields.
- File name digest is content-derived; records are immutable once committed.

## History ledger — `.noslop/history.jsonl`

Append-only; one ledger record per line (same shape as above). Produced by
`noslop compact`, which folds pending `.noslop/acks/*.json` records in and
deletes them. Union-merged across branches (`.gitattributes`:
`.noslop/history.jsonl merge=union`), so parallel compactions never conflict.

## Check output — `noslop check --json`

```json
{
  "passed": false,
  "files_checked": 15,
  "actor": "claude-code",
  "enforced": true,
  "tree_oid": "8f2c4b1e0a9d7c6b5a4f3e2d1c0b9a8f7e6d5c4b",
  "blocking": [
    {
      "id": "NOS-2",
      "file": "src/core/services/checker.rs",
      "target": "src/core/services/checker.rs",
      "message": "Verify acknowledgment matching logic correctness",
      "severity": "block",
      "acknowledged": false
    }
  ],
  "warnings": [],
  "acknowledged": []
}
```

- `enforced: false` means the actor is human and blocking checks are
  advisory (exit code 0). `--ci` forces `enforced: true` for any actor.
- `tree_oid` (optional, added within schema 1 as an additive field): the
  gate-time staged-tree object id (`git write-tree`). Joined with ledger
  record tree oids to distinguish self-correction from rubber-stamping.
  Omitted when git state is unavailable; payloads from older versions
  simply lack it.
- `check_set_version` (optional, added within schema 1 as an additive
  field): content hash of the cloud-distributed check set in force for
  this run, echoed from `GET /api/v1/repo/checks`. Absent when the repo
  has no `[remote]` binding — the governing rules then live entirely in
  the tree.
- `check_set_age_seconds` (optional, added within schema 1 as an additive
  field): seconds between the cloud check set being fetched and this run
  gating with it (0 = fetched during the run). The fetch is fail-open, so
  a run may legitimately gate on cached rules; this field lets consumers
  flag runs that used a stale set. Absent without a `[remote]` binding.
- `monitor` (optional, added within schema 1 as an additive field):
  surfacings of monitor-state cloud checks, same item shape as
  `blocking`. Recorded for promotion decisions; never agent-visible,
  never gating, omitted when empty.
- This payload is the check-run upload's `check` field, verbatim.

## Fire events — `.noslop/events.jsonl` (local, per-clone)

One JSON object per line, written by `noslop check` whenever a check
surfaces unacknowledged. Gitignored working state, NOT part of the stable
API: only `noslop stats` on the same clone reads it. Team-level aggregation
happens via the check-run payload, not this file.

```json
{
  "schema": 1,
  "check_id": "NOS-2",
  "file": "src/core/services/checker.rs",
  "severity": "block",
  "actor": "claude-code",
  "tree_oid": "8f2c4b1e0a9d7c6b5a4f3e2d1c0b9a8f7e6d5c4b",
  "created_at": "2026-07-03T00:53:11.000Z"
}
```

## Upload envelope — Action `upload-url` POST body

When the GitHub Action is configured with `upload-url`, it POSTs the check
payload wrapped with run coordinates. This is the hosted-ingestion contract.
The envelope is built by `noslop envelope` (the CLI is the single
serializer of this format).

```json
{
  "schema": 1,
  "repo": "noslop-sh/noslop",
  "sha": "8f2c4b1e…",
  "pr": "42",
  "base": "origin/main",
  "check": { "passed": false, "actor": "ci", "blocking": ["…"] },
  "ledger": [
    {
      "schema": 1,
      "check_id": "NOS-2",
      "message": "exact-ID matching only; added tests",
      "acknowledged_by": "claude-code",
      "created_at": "2026-07-02T00:53:11.000Z",
      "tree_oid": "8f2c4b1e0a9d7c6b5a4f3e2d1c0b9a8f7e6d5c4b"
    }
  ]
}
```

- `check` is the `noslop check --json` payload verbatim (see above).
- `ledger` (optional, added within schema 1 as an additive field): the
  repository's ledger records for the check ids this run touched — the
  acknowledgment justification, actor, timestamp, and ack-time tree oid.
  Consumers must accept envelopes without it (older CLI versions omit it).
  Unrelated ledger history never leaves the repository.
- Upload is telemetry, not truth: a failed upload never changes the gate
  verdict.

## Commit trailer — `Noslop-Ack: <check-id> | <message> | <actor>`

Cosmetic convenience for humans reading `git log`. Not durable across squash
merges; the ledger record is the source of truth.
