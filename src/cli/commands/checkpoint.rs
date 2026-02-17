//! Checkpoint command handler
//!
//! `noslop checkpoint [message]` stages all changes and creates a
//! safety commit bypassing hooks. Used before risky operations or
//! between review iterations.

use chrono::Local;

use noslop::adapters::GitVersionControl;
use noslop::core::ports::VersionControl;
use noslop::output::OutputMode;

/// Run the checkpoint command
///
/// If `message` is `None`, generates a timestamp-based default message.
pub fn checkpoint(message: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;

    let default_msg;
    let commit_msg = match message {
        Some(m) => m,
        None => {
            default_msg = format!("checkpoint: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
            &default_msg
        },
    };

    match vcs.commit_all(commit_msg)? {
        Some(sha) => {
            if mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "sha": sha,
                        "message": commit_msg,
                    })
                );
            } else {
                println!("Checkpoint: {sha} {commit_msg}");
            }
        },
        None => {
            if mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "sha": null,
                        "message": "Nothing to checkpoint (working tree clean).",
                    })
                );
            } else {
                println!("Nothing to checkpoint (working tree clean).");
            }
        },
    }

    Ok(())
}
