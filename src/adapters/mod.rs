//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - [`env`] - Actor detection from the process environment
//! - [`mod@file`] - JSON file acknowledgment staging storage
//! - [`gh`] - Review-history fetching via the GitHub CLI
//! - [`git`] - Git operations (hooks, staging, version control)
//! - [`ledger`] - Durable ack records in the tree (squash-proof)
//! - [`proposals`] - Staged check proposals awaiting review
//! - [`rules`] - Rules-file discovery (CLAUDE.md, AGENTS.md, .cursor/rules)
//! - [`runner`] - Agent CLI subprocess for mining prompts
//! - [`telemetry`] - Local check-fire event log for stats
//! - [`mod@toml`] - `.noslop.toml` file parsing and writing
//! - [`trailer`] - Commit trailer acknowledgment storage

pub mod env;
pub mod file;
pub mod gh;
pub mod git;
pub mod ledger;
pub mod proposals;
pub mod remote;
pub mod rules;
pub mod runner;
pub mod telemetry;
pub mod toml;
pub mod trailer;

// Re-export main types for convenience
pub use env::detect_actor;
pub use file::FileStore;
pub use git::{GitVersionControl, get_repo_name};
pub use toml::TomlCheckRepository;
pub use trailer::{TrailerAckStore, append_trailers};
