//! Common types and utilities shared across API handlers

use axum::http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::core::error::HyperTideError;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    /// Create a successful response with data
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn err(msg: impl Into<String>) -> Self {
        Self::err_with_code("error", msg)
    }

    pub fn err_with_code(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: msg.into(),
                details: None,
                request_id: Uuid::new_v4().to_string(),
            }),
        }
    }
}

pub fn map_error<T>(error: HyperTideError) -> (StatusCode, ApiResponse<T>) {
    let status = match &error {
        HyperTideError::Authentication(_) => StatusCode::UNAUTHORIZED,
        HyperTideError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        HyperTideError::Conflict(_) => StatusCode::CONFLICT,
        HyperTideError::NotFound(_) => StatusCode::NOT_FOUND,
        HyperTideError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
        HyperTideError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
        HyperTideError::Validation(_) => StatusCode::BAD_REQUEST,
        HyperTideError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (
        status,
        ApiResponse::err_with_code(error.code(), error.message().to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_authentication_error() {
        let (status, response) =
            map_error::<()>(HyperTideError::Authentication("Invalid API key".into()));
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let error = response.error.expect("error payload");
        assert_eq!(error.code, "authentication_failed");
    }

    #[test]
    fn maps_baseline_conflict_error() {
        let (status, response) =
            map_error::<()>(HyperTideError::Conflict("baseline mismatch".into()));
        assert_eq!(status, StatusCode::CONFLICT);
        let error = response.error.expect("error payload");
        assert_eq!(error.code, "conflict");
    }

    #[test]
    fn maps_lock_conflict_error() {
        let (status, response) = map_error::<()>(HyperTideError::Conflict("lock conflict".into()));
        assert_eq!(status, StatusCode::CONFLICT);
        let error = response.error.expect("error payload");
        assert_eq!(error.message, "lock conflict");
    }

    #[test]
    fn maps_object_not_found_error() {
        let (status, response) =
            map_error::<()>(HyperTideError::NotFound("Object not found".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        let error = response.error.expect("error payload");
        assert_eq!(error.code, "not_found");
    }
}
