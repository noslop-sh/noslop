//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - [`mod@file`] - JSON file acknowledgment staging storage
//! - [`git`] - Git operations (hooks, staging, version control)
//! - [`mod@toml`] - `.noslop.toml` file parsing and writing
//! - [`trailer`] - Commit trailer acknowledgment storage

pub mod file;
pub mod git;
pub mod toml;
pub mod trailer;

// Re-export main types for convenience
pub use file::FileStore;
pub use git::{GitVersionControl, get_repo_name};
pub use toml::TomlCheckRepository;
pub use trailer::{TrailerAckStore, append_trailers};
