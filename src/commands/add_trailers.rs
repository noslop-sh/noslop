//! Add attestation trailers to commit message
//!
//! This command is called by the prepare-commit-msg hook to append
//! attestation trailers to the commit message.

use std::fs;
use std::path::Path;

/// Add attestation trailers to commit message file
///
/// Called by prepare-commit-msg hook with the commit message file path.
/// Appends Noslop-Attest trailers from staged attestations.
pub fn add_trailers(commit_msg_file: &str) -> anyhow::Result<()> {
    add_trailers_in(Path::new("."), commit_msg_file)
}

/// Add attestation trailers in a specific directory (for testing)
fn add_trailers_in(base_dir: &Path, commit_msg_file: &str) -> anyhow::Result<()> {
    use crate::models::Attestation;

    let msg_path = Path::new(commit_msg_file);
    if !msg_path.exists() {
        anyhow::bail!("Commit message file not found: {commit_msg_file}");
    }

    // Load staged attestations from base_dir
    let attestations_file = base_dir.join(".noslop/staged-attestations.json");
    let attestations: Vec<Attestation> = if attestations_file.exists() {
        let json = fs::read_to_string(&attestations_file)?;
        serde_json::from_str(&json)?
    } else {
        Vec::new()
    };

    if attestations.is_empty() {
        // No attestations to add
        return Ok(());
    }

    // Read existing commit message
    let mut msg = fs::read_to_string(msg_path)?;

    // Ensure message ends with newline before trailers
    if !msg.ends_with('\n') {
        msg.push('\n');
    }

    // Add blank line before trailers if message is not empty
    if !msg.trim().is_empty() && !msg.ends_with("\n\n") {
        msg.push('\n');
    }

    // Append attestation trailers
    for attestation in &attestations {
        let trailer = format!(
            "Noslop-Attest: {} | {} | {}\n",
            attestation.assertion_id, attestation.message, attestation.attested_by
        );
        msg.push_str(&trailer);
    }

    // Write back to commit message file
    fs::write(msg_path, msg)?;

    Ok(())
}
