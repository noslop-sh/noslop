//! CLI layer for noslop
//!
//! This module contains the command-line interface:
//!
//! - [`app`] - CLI definitions and entry point
//! - [`commands`] - Command implementations

pub mod app;
pub mod commands;

// Re-export main entry point
pub use app::run;
