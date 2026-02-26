use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::api::{common::ApiResponse, middleware::authz};
use crate::core::{auth::Permission, checkpoint::CheckpointRecord};
use crate::AppState;

async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<(), (StatusCode, String)> {
    authz::require_permission(state, headers, permission)
        .await
        .map(|_| ())
}

pub async fn generate_checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<CheckpointRecord>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(service) = &state.checkpoint_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("checkpoint service unavailable")),
        );
    };

    match service.generate_checkpoint().await {
        Ok(checkpoint) => (StatusCode::CREATED, Json(ApiResponse::ok(checkpoint))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("failed to generate checkpoint: {error}"))),
        ),
    }
}

pub async fn latest_checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<CheckpointRecord>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Download).await
    {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(service) = &state.checkpoint_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("checkpoint service unavailable")),
        );
    };

    match service.latest_checkpoint().await {
        Ok(Some(checkpoint)) => (StatusCode::OK, Json(ApiResponse::ok(checkpoint))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("checkpoint not found")),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("failed to load checkpoint: {error}"))),
        ),
    }
}
