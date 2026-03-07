//! Business logic services

/// Review pipeline orchestrator.
pub mod pipeline;
/// Centralized prompt builder for agent invocations.
pub mod prompt;

pub use pipeline::ReviewPipeline;
pub use prompt::build_review_prompt;
