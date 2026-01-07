//! HTTP server adapters
//!
//! This module provides adapters that translate between HTTP frameworks
//! and the HTTP-agnostic API layer.
//!
//! Currently supported:
//! - `tiny_http` - Lightweight HTTP server for CLI use

#[cfg(feature = "ui")]
pub mod tiny_http;
