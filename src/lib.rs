//! noslop - Local code review for agent-generated code
//!
//! This library provides the core functionality for:
//! - Defining checks that must be reviewed when code changes
//! - Tracking findings from checks, scripts, agents, and humans
//! - Integrating with git hooks for enforcement
//!
//! # Architecture
//!
//! The library follows a hexagonal (ports & adapters) architecture:
//!
//! - [`core`] - Pure domain logic with no I/O dependencies
//!   - [`core::models`] - Domain types (Check, Finding, Review, Severity, Target)
//!   - [`core::ports`] - Trait interfaces for I/O operations
//!   - [`core::services`] - Pure business logic
//!
//! - [`adapters`] - I/O implementations of port traits
//!   - [`adapters::toml`] - `.noslop.toml` file handling (parser, writer, repository)
//!   - [`adapters::git`] - Git integration (hooks, staging, diff, checkpoint)
//!   - [`adapters::review`] - JSON file review storage (`FileReviewStore`)
//!   - [`adapters::agents`] - AI agent adapters (Claude, Codex) with output parsing
//!   - [`adapters::analyzers`] - Review analyzers (`ConventionAnalyzer`)

// Deny all clippy warnings in this crate
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
// Allow some pedantic lints that are too noisy or not applicable
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::cargo_common_metadata
)]

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Core hexagonal architecture modules
pub mod adapters;
pub mod core;

// Output formatting (used by CLI and tests)
pub mod output;

// Re-exports for convenience
pub use core::models::{
    AgentKind, Check, DismissReason, Finding, FindingNote, FindingSource, FindingStatus,
    ResolutionReason, Review, ReviewStatus, Severity, Span, Target,
};
pub use core::ports::{
    AnalyzerTier, CheckRepository, CommitInfo, ContextKind, DiffHunk, DiffLine, DiffLineKind,
    DiffStat, DiffStatus, FileDiff, ReviewAnalyzer, ReviewContext, ReviewStore, VersionControl,
};
