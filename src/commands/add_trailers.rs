//! Add verification trailers to commit message
//!
//! This command is called by the prepare-commit-msg hook to append
//! verification trailers to the commit message.

use std::fs;
use std::path::Path;

/// Add verification trailers to commit message file
///
/// Called by prepare-commit-msg hook with the commit message file path.
/// Appends Noslop-Verify trailers from staged verifications.
pub fn add_trailers(commit_msg_file: &str) -> anyhow::Result<()> {
    add_trailers_in(Path::new("."), commit_msg_file)
}

/// Add verification trailers in a specific directory (for testing)
fn add_trailers_in(base_dir: &Path, commit_msg_file: &str) -> anyhow::Result<()> {
    use crate::models::Verification;

    let msg_path = Path::new(commit_msg_file);
    if !msg_path.exists() {
        anyhow::bail!("Commit message file not found: {commit_msg_file}");
    }

    // Load staged verifications from base_dir
    let verifications_file = base_dir.join(".noslop/staged-verifications.json");
    let verifications: Vec<Verification> = if verifications_file.exists() {
        let json = fs::read_to_string(&verifications_file)?;
        serde_json::from_str(&json)?
    } else {
        Vec::new()
    };

    if verifications.is_empty() {
        // No verifications to add
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

    // Append verification trailers
    for verification in &verifications {
        let trailer = format!(
            "Noslop-Verify: {} | {} | {}\n",
            verification.check_id, verification.message, verification.verified_by
        );
        msg.push_str(&trailer);
    }

    // Write back to commit message file
    fs::write(msg_path, msg)?;

    Ok(())
}
