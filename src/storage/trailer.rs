//! Commit message trailer storage
//!
//! Stores attestations as git commit message trailers:
//!   Noslop-Attest: <assertion> | <message> | <by>
//!
//! This is the most portable format - visible in GitHub, GitLab, etc.

use std::process::Command;

use super::AttestationStore;
use super::file::FileStore;
use crate::models::Attestation;

const ATTEST_TRAILER: &str = "Noslop-Attest";

/// Attestation store using commit trailers
#[derive(Debug, Clone, Copy)]
pub struct TrailerAttestationStore;

impl TrailerAttestationStore {
    /// Create a new trailer attestation store
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for TrailerAttestationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationStore for TrailerAttestationStore {
    fn stage(&self, attestation: &Attestation) -> anyhow::Result<()> {
        let mut staged = FileStore::load_staged_attestations()?;
        staged.push(attestation.clone());
        FileStore::save_staged_attestations(&staged)?;
        Ok(())
    }

    fn staged(&self) -> anyhow::Result<Vec<Attestation>> {
        FileStore::load_staged_attestations()
    }

    fn clear_staged(&self) -> anyhow::Result<()> {
        FileStore::clear_staged_attestations()
    }

    fn format_trailers(&self, attestations: &[Attestation]) -> String {
        attestations
            .iter()
            .map(|a| {
                format!(
                    "{}: {} | {} | {}",
                    ATTEST_TRAILER,
                    a.assertion_id,
                    a.message.replace('|', "-"),
                    a.attested_by
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Attestation>> {
        let output = Command::new("git")
            .args(["log", "-1", "--format=%(trailers)", commit_sha])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let trailers = String::from_utf8_lossy(&output.stdout);
        let mut attestations = Vec::new();

        for line in trailers.lines() {
            if let Some(value) = line.strip_prefix(&format!("{ATTEST_TRAILER}: "))
                && let Some(attestation) = parse_attestation_trailer(value)
            {
                attestations.push(attestation);
            }
        }

        Ok(attestations)
    }
}

/// Parse an attestation from trailer format: "assertion | message | by"
fn parse_attestation_trailer(value: &str) -> Option<Attestation> {
    let parts: Vec<&str> = value.splitn(3, " | ").collect();
    if parts.len() >= 2 {
        Some(Attestation::new(
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
