//! tiny_http server adapter
//!
//! Handles routing, body parsing, and response conversion for tiny_http.

use std::io::Cursor;
#[allow(unused_imports)]
use std::io::Read as _;

use serde::{Serialize, de::DeserializeOwned};
use tiny_http::{Header, Method, Request, Response, StatusCode};

use noslop::api::{
    self, ApiError, ApiResponse, BlockerRequest, CreateCheckRequest, CreateConceptRequest,
    CreateTaskRequest, LinkBranchRequest, SelectConceptRequest, UpdateConceptRequest,
    UpdateConfigRequest, UpdateTaskRequest,
};

// =============================================================================
// REQUEST HANDLING
// =============================================================================

/// Handle an API request and return a response
///
/// This is the main routing function that maps URL paths to handlers.
pub fn handle_api_request(request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let path = request.url().to_string();
    let method = request.method().clone();

    // Normalize path to remove /v1/ prefix for backward compatibility
    // Supports both /api/v1/... (versioned) and /api/... (legacy)
    let api_path = path
        .strip_prefix("/api/v1")
        .or_else(|| path.strip_prefix("/api"))
        .unwrap_or(&path);

    // Route API requests
    match (&method, api_path) {
        // GET endpoints
        (&Method::Get, "/status") => handle_result(api::get_status()),
        (&Method::Get, "/tasks") => handle_result(api::list_tasks()),
        (&Method::Get, "/checks") => handle_result(api::list_checks()),
        (&Method::Get, "/workspace") => handle_result(api::get_workspace()),
        (&Method::Get, "/config") => handle_result(api::get_config()),
        (&Method::Get, "/concepts") => handle_result(api::list_concepts()),

        // PATCH /config - update config
        (&Method::Patch, "/config") => match read_json_body::<UpdateConfigRequest>(request) {
            Ok(req) => handle_result(api::update_config(&req)),
            Err(e) => error_response(&e),
        },

        // POST /tasks - create task
        (&Method::Post, "/tasks") => match read_json_body::<CreateTaskRequest>(request) {
            Ok(req) => handle_result(api::create_task(&req)),
            Err(e) => error_response(&e),
        },

        // POST /checks - create check
        (&Method::Post, "/checks") => match read_json_body::<CreateCheckRequest>(request) {
            Ok(req) => handle_result(api::create_check(&req)),
            Err(e) => error_response(&e),
        },

        // POST /concepts - create concept
        (&Method::Post, "/concepts") => match read_json_body::<CreateConceptRequest>(request) {
            Ok(req) => handle_result(api::create_concept(&req)),
            Err(e) => error_response(&e),
        },

        // POST /concepts/select - select current concept
        (&Method::Post, "/concepts/select") => {
            match read_json_body::<SelectConceptRequest>(request) {
                Ok(req) => handle_result(api::select_concept(&req)),
                Err(e) => error_response(&e),
            }
        },

        // Task detail: GET /tasks/{id}
        _ if method == Method::Get && api_path.starts_with("/tasks/") => {
            let id = api_path.strip_prefix("/tasks/").unwrap_or("");
            handle_result(api::get_task(id))
        },

        // Task start: POST /tasks/{id}/start
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/start") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/start"))
                .unwrap_or("");
            handle_result(api::start_task(id))
        },

        // Task done: POST /tasks/{id}/done
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/done") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/done"))
                .unwrap_or("");
            handle_result(api::complete_task(id))
        },

        // Task reset: POST /tasks/{id}/reset
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/reset") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/reset"))
                .unwrap_or("");
            handle_result(api::reset_task(id))
        },

        // Task backlog: POST /tasks/{id}/backlog
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/backlog") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/backlog"))
                .unwrap_or("");
            handle_result(api::backlog_task(id))
        },

        // Task link-branch: POST /tasks/{id}/link-branch
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/link-branch") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/link-branch"))
                .unwrap_or("");
            match read_json_body::<LinkBranchRequest>(request) {
                Ok(req) => handle_result(api::link_branch(id, &req)),
                Err(e) => error_response(&e),
            }
        },

        // Task block: POST /tasks/{id}/block
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/block") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/block"))
                .unwrap_or("");
            match read_json_body::<BlockerRequest>(request) {
                Ok(req) => handle_result(api::add_blocker(id, &req)),
                Err(e) => error_response(&e),
            }
        },

        // Task unblock: POST /tasks/{id}/unblock
        _ if method == Method::Post
            && api_path.starts_with("/tasks/")
            && api_path.ends_with("/unblock") =>
        {
            let id = api_path
                .strip_prefix("/tasks/")
                .and_then(|s| s.strip_suffix("/unblock"))
                .unwrap_or("");
            match read_json_body::<BlockerRequest>(request) {
                Ok(req) => handle_result(api::remove_blocker(id, &req)),
                Err(e) => error_response(&e),
            }
        },

        // Task update: PATCH /tasks/{id}
        _ if method == Method::Patch && api_path.starts_with("/tasks/") => {
            let id = api_path.strip_prefix("/tasks/").unwrap_or("");
            // Make sure it's not a sub-action like /tasks/id/something
            if !id.contains('/') {
                match read_json_body::<UpdateTaskRequest>(request) {
                    Ok(req) => handle_result(api::update_task(id, &req)),
                    Err(e) => error_response(&e),
                }
            } else {
                not_found_response(&format!("API endpoint not found: {} {}", method, api_path))
            }
        },

        // Concept update: PATCH /concepts/{id}
        _ if method == Method::Patch && api_path.starts_with("/concepts/") => {
            let id = api_path.strip_prefix("/concepts/").unwrap_or("");
            // Make sure it's not a sub-action like /concepts/id/something
            if !id.contains('/') {
                match read_json_body::<UpdateConceptRequest>(request) {
                    Ok(req) => handle_result(api::update_concept(id, &req)),
                    Err(e) => error_response(&e),
                }
            } else {
                not_found_response(&format!("API endpoint not found: {} {}", method, api_path))
            }
        },

        // Task delete: DELETE /tasks/{id}
        _ if method == Method::Delete && api_path.starts_with("/tasks/") => {
            let id = api_path.strip_prefix("/tasks/").unwrap_or("");
            // Make sure it's not a sub-action like /tasks/id/start
            if !id.contains('/') {
                handle_result(api::delete_task(id))
            } else {
                not_found_response(&format!("API endpoint not found: {} {}", method, api_path))
            }
        },

        // Concept delete: DELETE /concepts/{id}
        _ if method == Method::Delete && api_path.starts_with("/concepts/") => {
            let id = api_path.strip_prefix("/concepts/").unwrap_or("");
            if !id.contains('/') {
                handle_result(
                    api::delete_concept(id).map(|()| serde_json::json!({"deleted": true})),
                )
            } else {
                not_found_response(&format!("API endpoint not found: {} {}", method, api_path))
            }
        },

        // 404 for unknown API routes
        _ => not_found_response(&format!("API endpoint not found: {} {}", method, api_path)),
    }
}

// =============================================================================
// BODY PARSING
// =============================================================================

/// Read and parse JSON body from request
fn read_json_body<T: DeserializeOwned>(request: &mut Request) -> Result<T, ApiError> {
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|e| ApiError::bad_request(format!("Failed to read request body: {}", e)))?;

    serde_json::from_str(&body).map_err(|e| ApiError::bad_request(format!("Invalid JSON: {}", e)))
}

// =============================================================================
// RESPONSE CONVERSION
// =============================================================================

/// Convert a handler result to an HTTP response
fn handle_result<T: Serialize>(result: Result<T, ApiError>) -> Response<Cursor<Vec<u8>>> {
    match result {
        Ok(data) => success_response(data),
        Err(e) => error_response(&e),
    }
}

/// Create a successful JSON response
fn success_response<T: Serialize>(data: T) -> Response<Cursor<Vec<u8>>> {
    let response = ApiResponse::success(data);
    json_response(&response, 200)
}

/// Create an error JSON response with appropriate status code
fn error_response(error: &ApiError) -> Response<Cursor<Vec<u8>>> {
    let response = ApiResponse::<()>::error(error.code.as_str(), &error.message);
    json_response(&response, error.status_code())
}

/// Create a 404 not found response
fn not_found_response(message: &str) -> Response<Cursor<Vec<u8>>> {
    let response = ApiResponse::<()>::error("NOT_FOUND", message);
    json_response(&response, 404)
}

/// Serialize data to JSON response with status code
fn json_response<T: Serialize>(data: &T, status: u16) -> Response<Cursor<Vec<u8>>> {
    let json = serde_json::to_string(data).unwrap_or_else(|_| r#"{"success":false}"#.to_string());
    Response::from_data(json.into_bytes())
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        .with_status_code(StatusCode(status))
}
