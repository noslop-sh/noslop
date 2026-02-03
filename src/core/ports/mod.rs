//! Port traits (interfaces) for external dependencies
//!
//! These traits define the boundaries between core business logic
//! and external systems (filesystem, git, storage backends).
//!
//! Implementations live in the `adapters` module.
//!
//! ## Design Principle
//!
//! The core domain logic depends only on these traits, never on concrete
//! implementations. This enables:
//!
//! - **Testability**: Mock implementations for unit tests
//! - **Flexibility**: Swap implementations without changing business logic
//! - **Clarity**: Clear boundaries between layers

mod acknowledgment_store;
mod check_repo;
mod vcs;

pub use acknowledgment_store::{AcknowledgmentStore, StorageBackend};
pub use check_repo::CheckRepository;
pub use vcs::VersionControl;
