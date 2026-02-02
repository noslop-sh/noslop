//! Adapter implementations for port traits
//!
//! This module contains concrete implementations that handle I/O:
//!
//! - `toml/` - `.noslop.toml` file parsing and writing
//! - `git/` - Git operations (hooks, staging)
//! - `trailer/` - Commit trailer attestation storage
//! - `file/` - JSON file attestation storage

pub mod file;
pub mod git;
pub mod toml;
pub mod trailer;
