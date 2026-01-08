//! Verify command - provide verifications for checks

use noslop::output::OutputMode;

use noslop::models::Verification;
use noslop::storage;

/// Verify a check
pub fn verify(check_ref: &str, message: &str, _mode: OutputMode) -> anyhow::Result<()> {
    // Create verification
    // The check_ref can be the check message, target, or ID
    let verification = Verification::by_human(check_ref.to_string(), message.to_string());

    // Stage via storage abstraction
    let store = storage::verification_store();
    store.stage(&verification)?;

    println!("Staged verification:");
    println!("  For: {}", check_ref);
    println!("  Message: {}", message);
    println!("\nThis will be recorded as a commit trailer:");
    println!("  {}", store.format_trailers(&[verification]));
    println!("\nRun 'git commit' to finalize.");

    Ok(())
}
