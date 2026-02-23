//! Core domain logic for noslop
//!
//! This module contains pure business logic with no I/O dependencies.
//! All external interactions are abstracted through port traits.
//!
//! # Hexagonal Architecture
//!
//! ```text
//!          +-----------------------------------------------+
//!          |               Adapters (I/O)                  |
//!          |  +------+ +-----+ +--------+ +-------+       |
//!          |  | TOML | | Git | | Agents | | Review|       |
//!          |  +--+---+ +--+--+ +---+----+ +---+---+       |
//!          +-----+--------+--------+----------+----------+
//!                |        |        |          |
//!          +-----v--------v--------v----------v----------+
//!          |                Ports (Traits)                |
//!          |  CheckRepository   ReviewStore               |
//!          |  VersionControl    ReviewAnalyzer             |
//!          |  AgentConfig       AgentRuntime               |
//!          +------------------------+--------------------+
//!                                   |
//!          +------------------------v--------------------+
//!          |             Core (Pure Logic)                |
//!          |  +----------+  +-----------+                |
//!          |  |  Models  |  | Services  |                |
//!          |  |  Check   |  | Pipeline  |                |
//!          |  |  Feedback |  +-----------+                |
//!          |  |  Review  |                               |
//!          |  | Severity |                               |
//!          |  |  Target  |                               |
//!          |  |AgentKind |                               |
//!          |  +----------+                               |
//!          +---------------------------------------------+
//! ```
//!
//! # Modules
//!
//! - [`models`] - Domain types (`Check`, `Feedback`, `Review`, `Target`, `Severity`, `AgentKind`)
//! - [`services`] - Business logic (`ReviewPipeline` with tier-sorted fold semantics)
//! - [`ports`] - Trait definitions for external dependencies

pub mod models;
pub mod ports;
pub mod services;
