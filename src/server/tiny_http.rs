//! tiny_http server adapter
//!
//! Handles routing, body parsing, and response conversion for tiny_http.

use std::io::Cursor;
#[allow(unused_imports)]
use std::io::Read as _;

use serde::{Serialize, de::DeserializeOwned};
use tiny_http::{Header, Method, Request, Response, StatusCode};

use noslop::api::{self, ApiError, ApiResponse, CreateCheckRequest, CreateTaskRequest};

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
