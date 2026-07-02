//! Compact command - fold pending ack records into the history ledger

use noslop::adapters::ledger;

/// Fold `.noslop/acks/*.json` into `.noslop/history.jsonl`
///
/// Intended to run at merge time (or periodically). The history file is
/// union-merged, so parallel branches compact without conflicts.
pub fn compact() -> anyhow::Result<()> {
    let count = ledger::compact()?;
    if count == 0 {
        println!("No pending ack records to compact.");
    } else {
        println!("Compacted {count} ack record(s) into {}", ledger::HISTORY_FILE);
        println!("Changes are staged; commit to finalize.");
    }
    Ok(())
}
