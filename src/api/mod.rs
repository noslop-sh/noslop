//! HTTP-agnostic API layer
//!
//! This module provides typed request/response structures and pure business logic
//! handlers that can be used by any HTTP server implementation (`tiny_http`, axum, etc.)
//! or directly by clients (CLI, `VSCode` extension, etc.).
//!
//! ## Design
//!
//! - **Handlers are pure functions**: Take typed input, return `Result<T, ApiError>`
//! - **Types are framework-agnostic**: No HTTP types leak into this module
//! - **Errors carry HTTP semantics**: `ApiError` knows its status code for translation

mod error;
mod handlers;
mod types;

pub use error::ApiError;
pub use handlers::{
    complete_task, create_check, create_task, get_status, get_task, list_checks, list_tasks,
    start_task,
};
pub use types::{ApiResponse, CreateCheckRequest, CreateTaskRequest, EventsData};
