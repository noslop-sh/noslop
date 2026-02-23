//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.

mod agent;
mod check;
mod feedback;
mod primitives;
mod review;

pub use agent::AgentKind;
pub use check::Check;
pub use feedback::{DismissReason, Feedback, FeedbackNote, FeedbackStatus, ResolutionReason};
pub use primitives::{FeedbackSource, Severity, Span, Target};
pub use review::{Review, ReviewStatus};
