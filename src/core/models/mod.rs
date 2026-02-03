//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.
//!
//! - [`Check`] - "When this code changes, verify this"
//! - [`Acknowledgment`] - "I verified this because..."
//! - [`Severity`] - How strictly a check is enforced
//! - [`Target`] - A reference to code (path, glob, or fragment)
//! - [`Review`] - A code review session with comments
//! - [`ReviewComment`] - A comment that extends Check with diff position

mod acknowledgment;
mod check;
mod review;
mod severity;
mod target;

pub use acknowledgment::Acknowledgment;
pub use check::Check;
pub use review::{CommentStatus, DiffPosition, DiffSide, Review, ReviewComment, ReviewStatus};
pub use severity::Severity;
pub use target::{Fragment, GlobPattern, ParseError, PathSpec, Target};
