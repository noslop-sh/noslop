//! Durable acknowledgment ledger stored as files in the repository tree
//!
//! Commit trailers do not survive squash merges or rebases; file content
//! does (a squash preserves the final tree). Each acknowledgment is written
//! to `.noslop/acks/` and staged into the same commit as the change it
//! acknowledges. `noslop compact` later folds records into the append-only
//! `.noslop/history.jsonl` (merged with `merge=union`, so parallel branches
//! never conflict).
//!
//! Record format is versioned: see `docs/SCHEMA.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::core::models::Acknowledgment;

/// Version of the on-disk ack record and history line format
pub const SCHEMA_VERSION: u32 = 1;

/// Directory holding pending (not yet compacted) ack records
pub const ACKS_DIR: &str = ".noslop/acks";

/// Append-only compacted history (union-merged across branches)
pub const HISTORY_FILE: &str = ".noslop/history.jsonl";

/// A versioned acknowledgment record as stored in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerRecord {
    /// Record format version
    pub schema: u32,
    /// The acknowledgment itself
    #[serde(flatten)]
    pub ack: Acknowledgment,
}

/// Write an acknowledgment record into `.noslop/acks/` and stage it
///
/// The record lands in the same commit as the acknowledged change, so it
/// survives squash merges and rebases.
///
/// # Errors
///
/// Returns an error if the record cannot be written or staged.
pub fn record(ack: &Acknowledgment) -> anyhow::Result<PathBuf> {
    let record = LedgerRecord {
        schema: SCHEMA_VERSION,
        ack: ack.clone(),
    };
    let json = serde_json::to_string_pretty(&record)?;

    fs::create_dir_all(ACKS_DIR)?;
    let path = Path::new(ACKS_DIR).join(record_file_name(ack));
    fs::write(&path, &json)?;

    git_add(&path)?;
    Ok(path)
}

/// Fold all pending ack records into `.noslop/history.jsonl`
///
/// Returns the number of records compacted. Intended to run at merge time
/// (or periodically) so `.noslop/acks/` stays small.
///
/// # Errors
///
/// Returns an error if records cannot be read, appended, or staged.
pub fn compact() -> anyhow::Result<usize> {
    let acks_dir = Path::new(ACKS_DIR);
    if !acks_dir.exists() {
        return Ok(0);
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(acks_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();

    if entries.is_empty() {
        return Ok(0);
    }

    let mut history = String::new();
    for path in &entries {
        let content = fs::read_to_string(path)?;
        // Re-parse to guarantee one valid record per line
        let record: LedgerRecord = serde_json::from_str(&content)?;
        history.push_str(&serde_json::to_string(&record)?);
        history.push('\n');
    }

    let history_path = Path::new(HISTORY_FILE);
    let existing = if history_path.exists() {
        fs::read_to_string(history_path)?
    } else {
        String::new()
    };
    fs::write(history_path, format!("{existing}{history}"))?;
    git_add(history_path)?;

    for path in &entries {
        fs::remove_file(path)?;
        git_add(path)?; // stage the deletion
    }

    Ok(entries.len())
}

/// Load pending (not yet compacted) ledger records from `.noslop/acks/`.
///
/// During a pull request this is the branch's own acknowledgments —
/// compaction folds them into history at merge time — so CI reconciles
/// fired checks against exactly these.
///
/// # Errors
///
/// Returns an error only if an existing file cannot be read.
pub fn load_pending() -> anyhow::Result<Vec<Acknowledgment>> {
    Ok(pending_records()?.into_iter().map(|r| r.ack).collect())
}

/// Load every acknowledgment in the ledger: pending records plus history.
///
/// # Errors
///
/// Returns an error only if an existing file cannot be read.
pub fn load_all() -> anyhow::Result<Vec<Acknowledgment>> {
    Ok(load_all_records()?.into_iter().map(|r| r.ack).collect())
}

/// Load every full ledger record (schema wrapper included): pending plus
/// history. This is the shape the upload envelope ships to consumers.
///
/// Unparsable entries are skipped — the ledger accumulates records from
/// multiple schema-compatible versions and readers must not fail on one
/// bad line.
///
/// # Errors
///
/// Returns an error only if an existing file cannot be read.
pub fn load_all_records() -> anyhow::Result<Vec<LedgerRecord>> {
    let mut records = pending_records()?;

    let history_path = Path::new(HISTORY_FILE);
    if history_path.exists() {
        for line in fs::read_to_string(history_path)?.lines() {
            if let Ok(record) = serde_json::from_str::<LedgerRecord>(line) {
                records.push(record);
            }
        }
    }

    Ok(records)
}

fn pending_records() -> anyhow::Result<Vec<LedgerRecord>> {
    let acks_dir = Path::new(ACKS_DIR);
    if !acks_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(acks_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();

    let mut records = Vec::new();
    for path in entries {
        if let Ok(record) = serde_json::from_str::<LedgerRecord>(&fs::read_to_string(&path)?) {
            records.push(record);
        }
    }
    Ok(records)
}

/// Deterministic record file name: `<check-id>-<content-digest>.json`
fn record_file_name(ack: &Acknowledgment) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in ack
        .check_id
        .bytes()
        .chain(ack.message.bytes())
        .chain(ack.acknowledged_by.bytes())
        .chain(ack.created_at.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let safe_id: String = ack
        .check_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe_id}-{hash:012x}.json")
}

fn git_add(path: &Path) -> anyhow::Result<()> {
    // -f: ledger records must be tracked even when a repo ignores .noslop/
    let output = Command::new("git").args(["add", "-f", "--"]).arg(path).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to stage {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_file_name_is_deterministic_and_safe() {
        let ack = Acknowledgment::new(
            "NOS/2:weird id".to_string(),
            "message".to_string(),
            "claude-code".to_string(),
        );
        let a = record_file_name(&ack);
        let b = record_file_name(&ack);
        assert_eq!(a, b);
        assert!(a.starts_with("NOS_2_weird_id-"));
        assert_eq!(Path::new(&a).extension().and_then(|e| e.to_str()), Some("json"));
        assert!(!a.contains('/'));
    }

    #[test]
    fn ledger_record_serializes_with_schema_version() {
        let record = LedgerRecord {
            schema: SCHEMA_VERSION,
            ack: Acknowledgment::new("NOS-1".into(), "verified".into(), "claude-code".into()),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"schema\":1"));
        assert!(json.contains("\"check_id\":\"NOS-1\""));
        assert!(json.contains("\"acknowledged_by\":\"claude-code\""));
    }
}
