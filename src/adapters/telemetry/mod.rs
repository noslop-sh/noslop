//! Local check-fire telemetry
//!
//! Appends fire events to `.noslop/events.jsonl` — per-clone, gitignored
//! working state (like staged acks), NOT part of the tracked ledger. Stats
//! reads it to compute fire rates and the rubber-stamp join locally; the
//! future CI Action aggregates the team view from check-run payloads
//! instead.

use std::fs::OpenOptions;
use std::io::Write;

use crate::adapters::git::state_path;
use crate::core::models::CheckFireEvent;

const EVENTS_PATH: &str = ".noslop/events.jsonl";

/// Append fire events to the local telemetry log.
///
/// # Errors
///
/// Returns an error if the log cannot be written.
pub fn append_events(events: &[CheckFireEvent]) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }
    let path = state_path(EVENTS_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for event in events {
        writeln!(file, "{}", serde_json::to_string(event)?)?;
    }
    Ok(())
}

/// Load all fire events, skipping unparsable lines.
///
/// # Errors
///
/// Returns an error only if an existing log cannot be read.
pub fn load_events() -> anyhow::Result<Vec<CheckFireEvent>> {
    let path = state_path(EVENTS_PATH);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}
