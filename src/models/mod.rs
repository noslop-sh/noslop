//! Data models for noslop
//!
//! Core abstractions:
//! - Assertion: "When X changes, verify Y" (attached to paths)
//! - Attestation: "I verified Y because Z" (attached to commits)

pub mod assertion;
mod attestation;

pub use assertion::{Assertion, Severity};
pub use attestation::Attestation;
