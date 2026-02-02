//! Core domain logic for noslop
//!
//! This module contains pure business logic with no I/O dependencies.
//! All external interactions are abstracted through port traits.
//!
//! ## Architecture
//!
//! - `models/` - Domain types (Assertion, Attestation, Target, Severity)
//! - `services/` - Business logic orchestration
//! - `ports/` - Trait definitions for external dependencies

pub mod models;
pub mod ports;
pub mod services;
