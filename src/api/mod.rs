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
    add_blocker, backlog_task, complete_task, create_check, create_concept, create_task,
    delete_concept, delete_task, get_config, get_status, get_task, get_workspace, link_branch,
    list_checks, list_concepts, list_tasks, list_tasks_filtered, remove_blocker, reset_task,
    select_concept, start_task, update_concept, update_config, update_task,
};
pub use types::{
    ApiResponse, BlockerRequest, ConceptCreateData, ConceptInfo, ConceptsData, CreateCheckRequest,
    CreateConceptRequest, CreateTaskRequest, EventsData, LinkBranchRequest, SelectConceptRequest,
    UpdateConceptRequest, UpdateConfigRequest, UpdateTaskRequest,
};
