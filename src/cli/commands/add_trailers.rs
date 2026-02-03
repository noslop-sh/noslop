//! Add acknowledgment trailers to commit message
//!
//! This command is called by the commit-msg hook to append
//! acknowledgment trailers to the commit message.

use std::fs;
use std::path::Path;

/// Add acknowledgment trailers to commit message file
///
/// Called by commit-msg hook with the commit message file path.
/// Appends Noslop-Ack trailers from staged acknowledgments.
pub fn add_trailers(commit_msg_file: &str) -> anyhow::Result<()> {
    add_trailers_in(Path::new("."), commit_msg_file)
}

/// Add acknowledgment trailers in a specific directory (for testing)
fn add_trailers_in(base_dir: &Path, commit_msg_file: &str) -> anyhow::Result<()> {
    use noslop::core::models::Acknowledgment;

    let msg_path = Path::new(commit_msg_file);
    if !msg_path.exists() {
        anyhow::bail!("Commit message file not found: {commit_msg_file}");
    }

    // Load staged acknowledgments from base_dir
    let acks_file = base_dir.join(".noslop/staged-acks.json");
    let acks: Vec<Acknowledgment> = if acks_file.exists() {
        let json = fs::read_to_string(&acks_file)?;
        serde_json::from_str(&json)?
    } else {
        Vec::new()
    };

    if acks.is_empty() {
        // No acknowledgments to add
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

    // Append acknowledgment trailers
    for ack in &acks {
        let trailer =
            format!("Noslop-Ack: {} | {} | {}\n", ack.check_id, ack.message, ack.acknowledged_by);
        msg.push_str(&trailer);
    }

    // Write back to commit message file
    fs::write(msg_path, msg)?;

    Ok(())
}
