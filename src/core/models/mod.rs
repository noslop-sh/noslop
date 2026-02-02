//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.

mod assertion;
mod attestation;
mod severity;
mod target;

pub use assertion::Assertion;
pub use attestation::Attestation;
pub use severity::Severity;
pub use target::{Fragment, PathSpec, Target};
