//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - [`agents`] - Agent configuration generators and runtime invokers
//! - [`analyzers`] - Review analyzer implementations
//! - [`git`] - Git operations (hooks, staging, version control)
//! - [`review`] - JSON file review session storage
//! - [`mod@toml`] - `.noslop.toml` file parsing and writing

pub mod agents;
pub mod analyzers;
pub mod git;
pub mod review;
pub mod toml;

// Re-export main types for convenience
pub use agents::{get_agent_config, get_agent_runtime};
pub use analyzers::ConventionAnalyzer;
pub use git::{GitVersionControl, get_repo_name};
pub use review::FileReviewStore;
pub use toml::TomlCheckRepository;
