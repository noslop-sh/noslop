//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.
//!
//! - [`Check`] - "When this code changes, verify this"
//! - [`Acknowledgment`] - "I verified this because..."
//! - [`Actor`] - Who is committing or acknowledging (human or agent)
//! - [`Severity`] - How strictly a check is enforced
//! - [`Target`] - A reference to code (path, glob, or fragment)

mod acknowledgment;
mod actor;
mod check;
mod severity;
mod target;

pub use acknowledgment::Acknowledgment;
pub use actor::Actor;
pub use check::Check;
pub use severity::Severity;
pub use target::{Fragment, GlobPattern, ParseError, PathSpec, Target};
