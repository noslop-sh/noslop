//! Port traits (interfaces) for external dependencies
//!
//! These traits define the boundaries between core business logic
//! and external systems (filesystem, git, storage backends).
//!
//! Implementations live in the `adapters` module.

/// Agent configuration and runtime traits.
pub mod agent;
/// Review analyzer traits.
pub mod analyzer;
mod check_repo;
mod review_store;
mod vcs;
/// VCS port types (diff, commit, stat).
pub mod vcs_types;

pub use analyzer::{AnalyzerTier, ContextKind, ReviewAnalyzer, ReviewContext};
pub use check_repo::CheckRepository;
pub use review_store::ReviewStore;
pub use vcs::VersionControl;
pub use vcs_types::{CommitInfo, DiffHunk, DiffLine, DiffLineKind, DiffStat, DiffStatus, FileDiff};
