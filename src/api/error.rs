//! API error types with HTTP status code mapping

use serde::Serialize;

/// Error codes that map to HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Resource not found (404)
    NotFound,
    /// Invalid request (400)
    BadRequest,
    /// Internal server error (500)
    Internal,
}

impl ErrorCode {
    /// Get the HTTP status code for this error
    #[must_use]
    pub const fn status_code(self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::BadRequest => 400,
            Self::Internal => 500,
        }
    }

    /// Get the error code string
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::BadRequest => "BAD_REQUEST",
            Self::Internal => "INTERNAL_ERROR",
        }
    }
}

/// API error with code and message
#[derive(Debug, Clone)]
pub struct ApiError {
    /// Error code (determines HTTP status)
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
}

impl ApiError {
    /// Create a not found error
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
        }
    }

    /// Create a bad request error
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::BadRequest,
            message: message.into(),
        }
    }

    /// Create an internal error
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.code.status_code()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ApiError {}

/// Serializable error data for JSON responses
#[derive(Debug, Serialize)]
pub struct ApiErrorData {
    /// Error code string
    pub code: String,
    /// Human-readable message
    pub message: String,
}

impl From<&ApiError> for ApiErrorData {
    fn from(err: &ApiError) -> Self {
        Self {
            code: err.code.as_str().to_string(),
            message: err.message.clone(),
        }
    }
}
