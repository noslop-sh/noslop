//! Data models for noslop
//!
//! Core abstractions:
//! - Check: "When X changes, verify Y" (attached to paths)
//! - Verification: "I verified Y because Z" (attached to commits)
//! - Task: "What needs to be done" (with dependencies and status)

pub mod check;
pub mod task;
mod verification;

pub use check::{Check, Severity};
pub use verification::Verification;
// These are library exports used by binary and external consumers
#[allow(unused_imports)]
pub use task::{Priority, Task, TaskStatus};
