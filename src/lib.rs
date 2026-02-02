//! noslop - Pre-commit assertions with attestation tracking
//!
//! This library provides the core functionality for:
//! - Defining assertions that must be reviewed when code changes
//! - Tracking attestations that prove review happened
//! - Integrating with git hooks for enforcement
//!
//! # Architecture
//!
//! The library follows a hexagonal (ports & adapters) architecture:
//!
//! - [`core`] - Pure domain logic with no I/O dependencies
//!   - [`core::models`] - Domain types (Assertion, Attestation, Severity)
//!   - [`core::ports`] - Trait interfaces for I/O operations
//!   - [`core::services`] - Pure business logic (matching, checking)
//!
//! - [`adapters`] - I/O implementations of port traits
//!   - [`adapters::toml`] - TOML file handling for assertions
//!   - [`adapters::git`] - Git integration (hooks, staging)
//!   - [`adapters::trailer`] - Commit trailer storage for attestations
//!   - [`adapters::file`] - JSON file storage for staging
//!
//! - [`shared`] - Cross-cutting utilities
//!   - [`shared::parser`] - Code parsing utilities
//!   - [`shared::resolver`] - File resolution utilities
//!
//! # Example
//!
//! ```rust,ignore
//! use noslop::core::models::{Assertion, Attestation, Severity};
//! use noslop::adapters::TomlAssertionRepository;
//! use noslop::core::ports::AssertionRepository;
//!
//! // Create a repository for assertions
//! let repo = TomlAssertionRepository::current_dir()?;
//!
//! // Find assertions for changed files
//! let assertions = repo.find_for_files(&["src/main.rs".to_string()])?;
//! ```

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
pub mod shared;

// Output formatting (used by CLI and tests)
pub mod output;

// Storage abstraction (facade over adapters)
pub mod storage;

// Re-exports for convenience
pub use core::models::{Assertion, Attestation, Severity};
pub use core::ports::{AssertionRepository, AttestationStore, VersionControl};
pub use core::services::{CheckResult, check_assertions, matches_target};

// Re-exports for backwards compatibility
pub use shared::parser;
pub use shared::resolver;
