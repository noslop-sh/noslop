//! Ack command - provide acknowledgments for checks

use noslop::output::OutputMode;

use crate::noslop_file;
use noslop::adapters::{detect_actor, ledger, telemetry};
use noslop::core::models::Acknowledgment;
use noslop::storage;

/// Acknowledge a check by its exact ID
pub fn ack(check_ref: &str, message: &str, _mode: OutputMode) -> anyhow::Result<()> {
    // The referenced check must exist: acks against unknown IDs would be
    // silent rubber stamps that never match anything.
    let Some(check) = noslop_file::find_check_by_id(check_ref)? else {
        let known: Vec<String> =
            noslop_file::load_all_checks()?.into_iter().map(|c| c.id).collect();
        if known.is_empty() {
            anyhow::bail!(
                "No check with ID '{check_ref}'. No checks are defined yet; add one with 'noslop check add'."
            );
        }
        anyhow::bail!("No check with ID '{check_ref}'. Known check IDs: {}", known.join(", "));
    };

    let actor = detect_actor();
    // Copy the latest local fire event into the record so it is
    // self-contained evidence (fire tree + time vs ack tree + time) —
    // events.jsonl never leaves this clone. Best-effort: no event, no fields.
    let last_fire = telemetry::load_events()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.check_id == check.id)
        .max_by(|a, b| a.created_at.cmp(&b.created_at));
    let ack = Acknowledgment::by_actor(check.id.clone(), message.to_string(), &actor)
        .with_tree_oid(crate::git::staged::staged_tree_oid().ok())
        .with_fire(last_fire.as_ref().map(|e| e.tree_oid.clone()), last_fire.map(|e| e.created_at));

    // Stage via storage abstraction (drives the pre-commit gate and trailers)
    let store = storage::ack_store();
    store.stage(&ack)?;

    // Durable ledger record: staged into the same commit, survives squash
    let record_path = ledger::record(&ack)?;

    println!("Staged acknowledgment (as {}):", actor.name());
    println!("  For: {} - {}", check.id, check.message);
    println!("  Message: {}", message);
    println!("  Ledger: {}", record_path.display());
    println!("\nThis will be recorded as a commit trailer:");
    println!("  {}", store.format_trailers(&[ack]));
    println!("\nRun 'git commit' to finalize.");

    Ok(())
}
