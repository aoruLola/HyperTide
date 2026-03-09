//! Common types and utilities shared across API handlers

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

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
