//! Acknowledgment store port
//!
//! Defines the interface for staging and persisting acknowledgments.

use super::super::models::Acknowledgment;

/// Storage backend for acknowledgments
///
/// Implementations handle how acknowledgments are staged during development
/// and persisted in commits (e.g., via trailers, git notes, files).
#[cfg_attr(test, mockall::automock)]
pub trait AcknowledgmentStore: Send + Sync {
    /// Stage an acknowledgment (pending until commit)
    ///
    /// Staged acknowledgments are temporary and will be cleared after commit.
    fn stage(&self, ack: &Acknowledgment) -> anyhow::Result<()>;

    /// Get all staged acknowledgments
    fn staged(&self) -> anyhow::Result<Vec<Acknowledgment>>;

    /// Clear staged acknowledgments (called after commit succeeds)
    fn clear_staged(&self) -> anyhow::Result<()>;

    /// Format acknowledgments for commit message trailer
    ///
    /// Returns a string suitable for appending to a commit message.
    fn format_trailers(&self, acks: &[Acknowledgment]) -> String;

    /// Parse acknowledgments from a commit message
    ///
    /// Used to retrieve acknowledgment history from past commits.
    fn parse_from_commit(&self, commit_sha: &str) -> anyhow::Result<Vec<Acknowledgment>>;
}
