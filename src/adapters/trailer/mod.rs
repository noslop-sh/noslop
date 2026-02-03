//! Commit trailer acknowledgment storage
//!
//! Implements `AcknowledgmentStore` using git commit trailers.
//! Stores acknowledgments as:
//!   `Noslop-Ack: <check> | <message> | <by>`
//!
//! This is the most portable format - visible in GitHub, GitLab, etc.

use std::process::Command;

use crate::adapters::file::FileStore;
use crate::core::models::Acknowledgment;
use crate::core::ports::AcknowledgmentStore;

const ACK_TRAILER: &str = "Noslop-Ack";

/// Acknowledgment store using commit trailers
#[derive(Debug, Clone, Copy)]
pub struct TrailerAckStore;

impl TrailerAckStore {
    /// Create a new trailer acknowledgment store
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for TrailerAckStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AcknowledgmentStore for TrailerAckStore {
    fn stage(&self, ack: &Acknowledgment) -> anyhow::Result<()> {
        let mut staged = FileStore::load_staged_acks()?;
        staged.push(ack.clone());
        FileStore::save_staged_acks(&staged)?;
        Ok(())
    }

    fn staged(&self) -> anyhow::Result<Vec<Acknowledgment>> {
        FileStore::load_staged_acks()
    }

    fn clear_staged(&self) -> anyhow::Result<()> {
        FileStore::clear_staged_acks()
    }

    fn format_trailers(&self, acks: &[Acknowledgment]) -> String {
        acks.iter()
            .map(|a| {
                format!(
                    "{}: {} | {} | {}",
                    ACK_TRAILER,
                    a.check_id,
                    a.message.replace('|', "-"),
                    a.acknowledged_by
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Acknowledgment>> {
        let output = Command::new("git")
            .args(["log", "-1", "--format=%(trailers)", commit_sha])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let trailers = String::from_utf8_lossy(&output.stdout);
        let mut acks = Vec::new();

        for line in trailers.lines() {
            if let Some(value) = line.strip_prefix(&format!("{ACK_TRAILER}: "))
                && let Some(ack) = parse_ack_trailer(value)
            {
                acks.push(ack);
            }
        }

        Ok(acks)
    }
}

/// Parse an acknowledgment from trailer format: "check | message | by"
fn parse_ack_trailer(value: &str) -> Option<Acknowledgment> {
    let parts: Vec<&str> = value.splitn(3, " | ").collect();
    if parts.len() >= 2 {
        Some(Acknowledgment::new(
            parts[0].trim().to_string(),
            parts[1].trim().to_string(),
            parts.get(2).map_or("unknown", |s| s.trim()).to_string(),
        ))
    } else {
        None
    }
}

/// Helper to append trailers to a commit message
#[must_use]
pub fn append_trailers(message: &str, trailers: &str) -> String {
    if trailers.is_empty() {
        return message.to_string();
    }

    let message = message.trim_end();

    // Check if message already has trailers (ends with "Key: Value" lines)
    if message.lines().last().is_some_and(|l| l.contains(": ")) {
        format!("{message}\n{trailers}")
    } else {
        format!("{message}\n\n{trailers}")
    }
}
