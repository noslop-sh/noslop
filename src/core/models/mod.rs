//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.
//!
//! - [`Assertion`] - "When this code changes, verify this"
//! - [`Attestation`] - "I verified this because..."
//! - [`Severity`] - How strictly an assertion is enforced
//! - [`Target`] - A reference to code (path, glob, or fragment)

mod assertion;
mod attestation;
mod severity;
mod target;

pub use assertion::Assertion;
pub use attestation::Attestation;
pub use severity::Severity;
pub use target::{Fragment, GlobPattern, ParseError, PathSpec, Target};
