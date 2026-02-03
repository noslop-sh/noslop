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
//!                  │  CheckRepository                     │
//!                  │  AcknowledgmentStore                 │
//!                  │  VersionControl                      │
//!                  └─────────────────┬───────────────────┘
//!                                    │
//!                  ┌─────────────────▼───────────────────┐
//!                  │           Core (Pure Logic)         │
//!                  │  ┌──────────┐  ┌────────────────┐   │
//!                  │  │  Models  │  │    Services    │   │
//!                  │  │  Check   │  │  check_items   │   │
//!                  │  │  Ack     │  │ matches_target │   │
//!                  │  │ Target   │  └────────────────┘   │
//!                  │  │ Severity │                       │
//!                  │  └──────────┘                       │
//!                  └─────────────────────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`models`] - Domain types (Check, Acknowledgment, Target, Severity)
//! - [`services`] - Business logic (`check_items`, `matches_target`)
//! - [`ports`] - Trait definitions for external dependencies
//!
//! # Usage
//!
//! The core module can be used independently of any I/O:
//!
//! ```
//! use noslop::core::models::{Check, Acknowledgment, Severity};
//! use noslop::core::services::check_items;
//!
//! // Create test data
//! let check = Check::new(
//!     Some("TEST-1".to_string()),
//!     "*.rs".to_string(),
//!     "Review Rust code".to_string(),
//!     Severity::Block,
//! );
//!
//! let ack = Acknowledgment::new(
//!     "TEST-1".to_string(),
//!     "Reviewed".to_string(),
//!     "human".to_string(),
//! );
//!
//! // Check items - pure function, no I/O
//! let result = check_items(
//!     &[(check, "src/main.rs".to_string())],
//!     &[ack],
//!     1,
//! );
//!
//! assert!(result.passed);
//! ```

pub mod models;
pub mod ports;
pub mod services;
