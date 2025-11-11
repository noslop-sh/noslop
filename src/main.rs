//! noslop - A CLI tool to maintain high code and documentation quality in AI-assisted
//! development
//!
//! This tool helps engineering teams track provenance of code (especially AI-generated),
//! enforce quality through linting and documentation checks, and provides an extensible
//! plugin ecosystem.

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

/// Main entry point for the noslop CLI
fn main() {
    println!("noslop v{}", env!("CARGO_PKG_VERSION"));
    println!("A CLI tool for maintaining code quality in AI-assisted development");
}
