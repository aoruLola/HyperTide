//! Storage API Handlers
//! HTTP endpoints for file upload/download operations

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::core::storage::StorageManager;
use crate::api::common::ApiResponse;

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub hash: String,
    pub size_bytes: u64,
    pub original_path: String,
}

#[derive(Debug, Deserialize)]
pub struct HashQuery {
    pub hash: String,
}

/// POST /api/upload
/// Upload a file via multipart form
pub async fn upload_file(
    State(manager): State<StorageManager>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<UploadResponse>>) {
    // Get the first file field
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err("No file provided")),
            )
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!("Multipart error: {}", e))),
            )
        }
    };

    let filename = field.file_name().unwrap_or("unknown").to_string();
    
    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err(format!("Failed to read file: {}", e))),
            )
        }
    };

    match manager.store(&data, &filename).await {
        Ok(stored) => (
            StatusCode::OK,
            Json(ApiResponse::ok(UploadResponse {
                hash: stored.hash,
                size_bytes: stored.size_bytes,
                original_path: stored.original_path,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(e)),
        ),
    }
}

/// GET /api/download/:hash
/// Download a file by its hash
pub async fn download_file(
    State(manager): State<StorageManager>,
    Path(hash): Path<String>,
) -> Response {
    match manager.retrieve(&hash).await {
        Ok(data) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", hash))
                .body(Body::from(data))
                .unwrap()
        }
        Err(e) => {
            (StatusCode::NOT_FOUND, e).into_response()
        }
    }
}

/// GET /api/exists/:hash
/// Check if a file exists
pub async fn check_exists(
    State(manager): State<StorageManager>,
    Path(hash): Path<String>,
) -> Json<ApiResponse<bool>> {
    let exists = manager.exists(&hash).await;
    Json(ApiResponse::ok(exists))
}

/// GET /api/hash
/// Calculate hash of provided data (for client-side deduplication check)
#[derive(Debug, Deserialize)]
pub struct HashRequest {
    pub data: String, // Base64 encoded
}

#[derive(Debug, Serialize)]
pub struct HashResponse {
    pub hash: String,
}

pub async fn calculate_hash(
    Json(payload): Json<HashRequest>,
) -> Json<ApiResponse<HashResponse>> {
    use base64::Engine;
    
    match base64::engine::general_purpose::STANDARD.decode(&payload.data) {
        Ok(data) => {
            let hash = StorageManager::calculate_hash(&data);
            Json(ApiResponse::ok(HashResponse { hash }))
        }
        Err(e) => Json(ApiResponse::err(format!("Invalid base64: {}", e))),
    }
}

