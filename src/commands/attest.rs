//! Attest command - provide attestations for assertions

use noslop::output::OutputMode;

use crate::models::Attestation;
use crate::storage;

/// Attest to an assertion
pub fn attest(assertion_ref: &str, message: &str, _mode: OutputMode) -> anyhow::Result<()> {
    // Create attestation
    // The assertion_ref can be the assertion message, target, or ID
    let attestation = Attestation::by_human(assertion_ref.to_string(), message.to_string());

    // Stage via storage abstraction
    let store = storage::attestation_store();
    store.stage(&attestation)?;

    println!("Staged attestation:");
    println!("  For: {}", assertion_ref);
    println!("  Message: {}", message);
    println!("\nThis will be recorded as a commit trailer:");
    println!("  {}", store.format_trailers(&[attestation]));
    println!("\nRun 'git commit' to finalize.");

    Ok(())
}
