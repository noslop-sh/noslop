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
  "created_at": "2026-07-02T00:53:11.000Z"
}
```

- `acknowledged_by` is the detected actor: `human`, an agent name
  (`claude-code`, `cursor`, `codex`, `aider`, `github-actions`, `ci`,
  `unknown-agent`), or the explicit `NOSLOP_ACTOR` value.
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

## Commit trailer — `Noslop-Ack: <check-id> | <message> | <actor>`

Cosmetic convenience for humans reading `git log`. Not durable across squash
merges; the ledger record is the source of truth.
