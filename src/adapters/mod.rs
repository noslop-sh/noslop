//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - [`git`] - Git operations (hooks, staging, version control)
//! - [`review`] - JSON file review session storage
//! - [`mod@toml`] - `.noslop.toml` file parsing and writing

pub mod git;
pub mod review;
pub mod toml;

// Re-export main types for convenience
pub use git::{GitVersionControl, get_repo_name};
pub use review::FileReviewStore;
pub use toml::TomlCheckRepository;
