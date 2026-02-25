//! Lock API Handlers
//! HTTP endpoints for file locking operations

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::core::lock::{FileLock, LockManager};
use crate::api::common::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct LockRequest {
    pub file_path: String,
    pub owner_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UnlockRequest {
    pub file_path: String,
    pub owner_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ForceUnlockRequest {
    pub file_path: String,
}

/// POST /api/lock
/// Request a lock on a file
pub async fn lock_file(
    State(manager): State<LockManager>,
    Json(payload): Json<LockRequest>,
) -> (StatusCode, Json<ApiResponse<FileLock>>) {
    match manager.try_lock(payload.file_path, payload.owner_id) {
        Ok(lock) => (StatusCode::OK, Json(ApiResponse::ok(lock))),
        Err(e) => (StatusCode::CONFLICT, Json(ApiResponse::err(e))),
    }
}

/// DELETE /api/unlock
/// Release a lock (only owner can unlock)
pub async fn unlock_file(
    State(manager): State<LockManager>,
    Json(payload): Json<UnlockRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    match manager.unlock(&payload.file_path, &payload.owner_id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::ok(()))),
        Err(e) => (StatusCode::FORBIDDEN, Json(ApiResponse::err(e))),
    }
}

/// POST /api/break-lock
/// Admin force unlock (bypasses ownership check)
pub async fn force_unlock_file(
    State(manager): State<LockManager>,
    Json(payload): Json<ForceUnlockRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    let removed = manager.force_unlock(&payload.file_path);
    if removed {
        (StatusCode::OK, Json(ApiResponse::ok(true)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::err("File was not locked")))
    }
}

/// GET /api/locks
/// List all current locks
pub async fn list_locks(
    State(manager): State<LockManager>,
) -> Json<ApiResponse<Vec<FileLock>>> {
    let locks = manager.list_locks();
    Json(ApiResponse::ok(locks))
}
