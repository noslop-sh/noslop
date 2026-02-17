//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.

mod agent;
mod check;
mod finding;
mod primitives;
mod review;

pub use agent::AgentKind;
pub use check::Check;
pub use finding::{Finding, FindingStatus};
pub use primitives::{FindingSource, Severity, Span, Target};
pub use review::{Review, ReviewStatus};
