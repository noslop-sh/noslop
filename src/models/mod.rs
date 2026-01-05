//! Data models for noslop
//!
//! Core abstractions:
//! - Assertion: "When X changes, verify Y" (attached to paths)
//! - Attestation: "I verified Y because Z" (attached to commits)
//! - Task: "What needs to be done" (with dependencies and status)

pub mod assertion;
mod attestation;
pub mod task;

pub use assertion::{Assertion, Severity};
pub use attestation::Attestation;
// These are library exports used by binary and external consumers
#[allow(unused_imports)]
pub use task::{Priority, Task, TaskStatus};
