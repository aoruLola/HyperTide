//! Lock API Handlers
//! HTTP endpoints for file locking operations

use crate::api::common::ApiResponse;
use crate::api::middleware::authz;
use crate::core::auth::Permission;
use crate::core::lock::FileLock;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::json;

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
pub struct RenewLockRequest {
    pub file_path: String,
    pub owner_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ForceUnlockRequest {
    pub file_path: String,
}

async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<(), (StatusCode, String)> {
    authz::require_permission(state, headers, permission)
        .await
        .map(|_| ())?;
    Ok(())
}

/// POST /v2/locks/acquire
/// Request a lock on a file
pub async fn lock_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LockRequest>,
) -> (StatusCode, Json<ApiResponse<FileLock>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Lock).await {
        return (status, Json(ApiResponse::err(message)));
    }

    match state
        .lock_manager
        .try_lock(payload.file_path, payload.owner_id)
        .await
    {
        Ok(lock) => {
            if let Some(event_store) = &state.event_store {
                if let Err(error) = event_store
                    .append(
                        "LOCK_ACQUIRED",
                        &lock.owner_id,
                        None,
                        None,
                        json!({ "file_path": lock.file_path, "lease_expires_at": lock.lease_expires_at }),
                    )
                    .await
                {
                    tracing::warn!("failed to append lock acquire event: {error}");
                }
            }
            (StatusCode::OK, Json(ApiResponse::ok(lock)))
        }
        Err(e) => (StatusCode::CONFLICT, Json(ApiResponse::err(e))),
    }
}

/// DELETE /v2/locks/release
/// Release a lock (only owner can unlock)
pub async fn unlock_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UnlockRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Lock).await {
        return (status, Json(ApiResponse::err(message)));
    }

    match state
        .lock_manager
        .unlock(&payload.file_path, &payload.owner_id)
        .await
    {
        Ok(_) => {
            if let Some(event_store) = &state.event_store {
                if let Err(error) = event_store
                    .append(
                        "LOCK_RELEASED",
                        &payload.owner_id,
                        None,
                        None,
                        json!({ "file_path": payload.file_path }),
                    )
                    .await
                {
                    tracing::warn!("failed to append lock release event: {error}");
                }
            }
            (StatusCode::OK, Json(ApiResponse::ok(())))
        }
        Err(e) => (StatusCode::FORBIDDEN, Json(ApiResponse::err(e))),
    }
}

/// POST /v2/locks/renew
/// Renew lock lease (owner only)
pub async fn renew_lock_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RenewLockRequest>,
) -> (StatusCode, Json<ApiResponse<FileLock>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Lock).await {
        return (status, Json(ApiResponse::err(message)));
    }

    match state
        .lock_manager
        .renew_lock(&payload.file_path, &payload.owner_id)
        .await
    {
        Ok(lock) => {
            if let Some(event_store) = &state.event_store {
                if let Err(error) = event_store
                    .append(
                        "LOCK_RENEWED",
                        &payload.owner_id,
                        None,
                        None,
                        json!({ "file_path": payload.file_path, "lease_expires_at": lock.lease_expires_at }),
                    )
                    .await
                {
                    tracing::warn!("failed to append lock renew event: {error}");
                }
            }
            (StatusCode::OK, Json(ApiResponse::ok(lock)))
        }
        Err(e) => (StatusCode::FORBIDDEN, Json(ApiResponse::err(e))),
    }
}

/// POST /v2/locks/force-release
/// Admin force unlock (bypasses ownership check)
pub async fn force_unlock_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ForceUnlockRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    match state.lock_manager.force_unlock(&payload.file_path).await {
        Ok(true) => {
            if let Some(event_store) = &state.event_store {
                if let Err(error) = event_store
                    .append(
                        "LOCK_FORCE_RELEASED",
                        "system-admin",
                        None,
                        None,
                        json!({ "file_path": payload.file_path }),
                    )
                    .await
                {
                    tracing::warn!("failed to append lock force-release event: {error}");
                }
            }
            (StatusCode::OK, Json(ApiResponse::ok(true)))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("File was not locked")),
        ),
        Err(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(message)),
        ),
    }
}

/// GET /v2/locks/acquires
/// List all current locks
pub async fn list_locks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<Vec<FileLock>>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Lock).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let locks = state.lock_manager.list_locks();
    (StatusCode::OK, Json(ApiResponse::ok(locks)))
}
