//! Core domain logic for noslop
//!
//! This module contains pure business logic with no I/O dependencies.
//! All external interactions are abstracted through port traits.
//!
//! # Hexagonal Architecture
//!
//! The core follows hexagonal (ports & adapters) architecture:
//!
//! ```text
//!                  ┌─────────────────────────────────────┐
//!                  │           Adapters (I/O)            │
//!                  │  ┌─────┐  ┌─────┐  ┌─────────────┐  │
//!                  │  │TOML │  │ Git │  │   Trailer   │  │
//!                  │  └──┬──┘  └──┬──┘  └──────┬──────┘  │
//!                  └─────┼───────┼─────────────┼─────────┘
//!                        │       │             │
//!                  ┌─────▼───────▼─────────────▼─────────┐
//!                  │              Ports (Traits)          │
//!                  │  AssertionRepository                 │
//!                  │  AttestationStore                    │
//!                  │  VersionControl                      │
//!                  └─────────────────┬───────────────────┘
//!                                    │
//!                  ┌─────────────────▼───────────────────┐
//!                  │           Core (Pure Logic)         │
//!                  │  ┌──────────┐  ┌────────────────┐   │
//!                  │  │  Models  │  │    Services    │   │
//!                  │  │Assertion │  │ check_assertions│  │
//!                  │  │Attestation│ │ matches_target │   │
//!                  │  │ Target   │  └────────────────┘   │
//!                  │  │ Severity │                       │
//!                  │  └──────────┘                       │
//!                  └─────────────────────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`models`] - Domain types (Assertion, Attestation, Target, Severity)
//! - [`services`] - Business logic (`check_assertions`, `matches_target`)
//! - [`ports`] - Trait definitions for external dependencies
//!
//! # Usage
//!
//! The core module can be used independently of any I/O:
//!
//! ```
//! use noslop::core::models::{Assertion, Attestation, Severity};
//! use noslop::core::services::check_assertions;
//!
//! // Create test data
//! let assertion = Assertion::new(
//!     Some("TEST-1".to_string()),
//!     "*.rs".to_string(),
//!     "Review Rust code".to_string(),
//!     Severity::Block,
//! );
//!
//! let attestation = Attestation::new(
//!     "TEST-1".to_string(),
//!     "Reviewed".to_string(),
//!     "human".to_string(),
//! );
//!
//! // Check assertions - pure function, no I/O
//! let result = check_assertions(
//!     &[(assertion, "src/main.rs".to_string())],
//!     &[attestation],
//!     1,
//! );
//!
//! assert!(result.passed);
//! ```

pub mod models;
pub mod ports;
pub mod services;
