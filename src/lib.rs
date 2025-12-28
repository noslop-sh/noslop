//! noslop - A CLI tool to maintain high code and documentation quality in AI-assisted
//! development
//!
//! This library provides the core functionality for tracking code provenance,
//! enforcing quality checks, and managing an extensible plugin ecosystem.

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

pub mod models;
pub mod output;
pub mod parser;
pub mod resolver;
pub mod storage;
pub mod target;
