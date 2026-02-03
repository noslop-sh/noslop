//! Ack command - provide acknowledgments for checks

use noslop::output::OutputMode;

use noslop::core::models::Acknowledgment;
use noslop::storage;

/// Acknowledge a check
pub fn ack(check_ref: &str, message: &str, _mode: OutputMode) -> anyhow::Result<()> {
    // Create acknowledgment
    // The check_ref can be the check message, target, or ID
    let ack = Acknowledgment::by_human(check_ref.to_string(), message.to_string());

    // Stage via storage abstraction
    let store = storage::ack_store();
    store.stage(&ack)?;

    println!("Staged acknowledgment:");
    println!("  For: {}", check_ref);
    println!("  Message: {}", message);
    println!("\nThis will be recorded as a commit trailer:");
    println!("  {}", store.format_trailers(&[ack]));
    println!("\nRun 'git commit' to finalize.");

    Ok(())
}
