//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - [`file`] - JSON file attestation staging storage
//! - [`git`] - Git operations (hooks, staging, version control)
//! - [`toml`] - `.noslop.toml` file parsing and writing
//! - [`trailer`] - Commit trailer attestation storage

pub mod file;
pub mod git;
pub mod toml;
pub mod trailer;

// Re-export main types for convenience
pub use file::FileStore;
pub use git::{GitVersionControl, get_repo_name};
pub use toml::TomlAssertionRepository;
pub use trailer::{TrailerAttestationStore, append_trailers};
