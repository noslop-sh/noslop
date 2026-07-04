//! Build the upload envelope for hosted ingestion
//!
//! Wraps a `noslop check --json` payload with run coordinates and the
//! run's ledger records. Replaces the jq assembly that previously lived in
//! the GitHub Action, so the CLI is the single serializer of the envelope
//! contract (`docs/SCHEMA.md`).

use std::collections::HashSet;
use std::fs;

use noslop::adapters::ledger;
use noslop::output::{ENVELOPE_SCHEMA, UploadEnvelope};

/// Emit the upload envelope for a completed check run to stdout.
///
/// The check payload is embedded verbatim — this command never reinterprets
/// the gate verdict. Ledger records are filtered to the check ids the run
/// touched so unrelated history never leaves the repository.
///
/// # Errors
///
/// Returns an error if the check payload cannot be read or is not the JSON
/// object produced by `noslop check --json`.
pub fn envelope(
    check_json: &str,
    repo: &str,
    sha: &str,
    pr: &str,
    base: &str,
) -> anyhow::Result<()> {
    let check: serde_json::Value = serde_json::from_str(&fs::read_to_string(check_json)?)?;
    if !check.is_object() {
        anyhow::bail!("{check_json} is not a noslop check --json payload");
    }

    let touched = touched_check_ids(&check);
    let ledger = ledger::load_all_records()?
        .into_iter()
        .filter(|record| touched.contains(record.ack.check_id.as_str()))
        .collect();

    let envelope = UploadEnvelope {
        schema: ENVELOPE_SCHEMA,
        repo: repo.to_string(),
        sha: sha.to_string(),
        pr: pr.to_string(),
        base: base.to_string(),
        check,
        ledger,
    };

    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

/// Check ids present in the payload's blocking/warnings/acknowledged arrays.
fn touched_check_ids(check: &serde_json::Value) -> HashSet<String> {
    ["blocking", "warnings", "acknowledged"]
        .iter()
        .filter_map(|key| check.get(key)?.as_array())
        .flatten()
        .filter_map(|item| item.get("id")?.as_str())
        .map(ToString::to_string)
        .collect()
}
