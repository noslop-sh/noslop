//! Commit message trailer storage
//!
//! Stores verifications as git commit message trailers:
//!   Noslop-Verify: <check> | <message> | <by>
//!
//! This is the most portable format - visible in GitHub, GitLab, etc.

use std::process::Command;

use super::VerificationStore;
use super::file::FileStore;
use crate::models::Verification;

const VERIFY_TRAILER: &str = "Noslop-Verify";

/// Verification store using commit trailers
#[derive(Debug, Clone, Copy)]
pub struct TrailerVerificationStore;

impl TrailerVerificationStore {
    /// Create a new trailer verification store
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for TrailerVerificationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStore for TrailerVerificationStore {
    fn stage(&self, verification: &Verification) -> anyhow::Result<()> {
        let mut staged = FileStore::load_staged_verifications()?;
        staged.push(verification.clone());
        FileStore::save_staged_verifications(&staged)?;
        Ok(())
    }

    fn staged(&self) -> anyhow::Result<Vec<Verification>> {
        FileStore::load_staged_verifications()
    }

    fn clear_staged(&self) -> anyhow::Result<()> {
        FileStore::clear_staged_verifications()
    }

    fn format_trailers(&self, verifications: &[Verification]) -> String {
        verifications
            .iter()
            .map(|v| {
                format!(
                    "{}: {} | {} | {}",
                    VERIFY_TRAILER,
                    v.check_id,
                    v.message.replace('|', "-"),
                    v.verified_by
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Verification>> {
        let output = Command::new("git")
            .args(["log", "-1", "--format=%(trailers)", commit_sha])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let trailers = String::from_utf8_lossy(&output.stdout);
        let mut verifications = Vec::new();

        for line in trailers.lines() {
            if let Some(value) = line.strip_prefix(&format!("{VERIFY_TRAILER}: "))
                && let Some(verification) = parse_verification_trailer(value)
            {
                verifications.push(verification);
            }
        }

        Ok(verifications)
    }
}

/// Parse a verification from trailer format: "check | message | by"
fn parse_verification_trailer(value: &str) -> Option<Verification> {
    let parts: Vec<&str> = value.splitn(3, " | ").collect();
    if parts.len() >= 2 {
        Some(Verification::new(
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
