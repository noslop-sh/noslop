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

mod assertion_repo;
mod attestation_store;
mod vcs;

pub use assertion_repo::AssertionRepository;
pub use attestation_store::{AttestationStore, StorageBackend};
pub use vcs::VersionControl;
