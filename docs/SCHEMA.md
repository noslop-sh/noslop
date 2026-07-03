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
- This payload is the basis for the future CI check-run upload.

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

## Commit trailer — `Noslop-Ack: <check-id> | <message> | <actor>`

Cosmetic convenience for humans reading `git log`. Not durable across squash
merges; the ledger record is the source of truth.
