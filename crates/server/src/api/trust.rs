use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use crate::api::{common::ApiResponse, middleware::authz};
use crate::core::{
    audit_chain::{AuditChainEntry, AuditVerifyResult},
    auth::Permission,
    checkpoint::CheckpointRecord,
    compliance::RetentionPolicy,
    replay::{ReplayReadinessReport, ReplayVerification},
    witness::{WitnessReceipt, WitnessSummary, WitnessTopology},
};
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
            Json(ApiResponse::err(format!(
                "failed to generate checkpoint: {error}"
            ))),
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
            Json(ApiResponse::err(format!(
                "failed to load checkpoint: {error}"
            ))),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct AttestRequest {
    pub witness_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WitnessSummaryQuery {
    pub checkpoint_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AuditExportQuery {
    pub limit: Option<i64>,
    pub before_seq: Option<i64>,
    pub action: Option<String>,
    pub actor_id: Option<String>,
}

pub async fn attest_checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(checkpoint_id): Path<String>,
    Json(payload): Json<AttestRequest>,
) -> (StatusCode, Json<ApiResponse<WitnessReceipt>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }
    let Some(service) = &state.witness_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("witness service unavailable")),
        );
    };

    match service
        .attest(&checkpoint_id, payload.witness_id.as_deref())
        .await
    {
        Ok(receipt) => (StatusCode::OK, Json(ApiResponse::ok(receipt))),
        Err(message) => (StatusCode::BAD_REQUEST, Json(ApiResponse::err(message))),
    }
}

pub async fn witness_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WitnessSummaryQuery>,
) -> (StatusCode, Json<ApiResponse<WitnessSummary>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Download).await
    {
        return (status, Json(ApiResponse::err(message)));
    }
    let Some(service) = &state.witness_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("witness service unavailable")),
        );
    };

    match service.summary(&query.checkpoint_id).await {
        Ok(summary) => (StatusCode::OK, Json(ApiResponse::ok(summary))),
        Err(message) => (StatusCode::BAD_REQUEST, Json(ApiResponse::err(message))),
    }
}

pub async fn verify_audit_chain(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<AuditVerifyResult>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(audit_chain) = &state.audit_chain else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("audit chain unavailable")),
        );
    };

    match audit_chain.verify_chain().await {
        Ok(result) => (StatusCode::OK, Json(ApiResponse::ok(result))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "failed to verify audit chain: {error}"
            ))),
        ),
    }
}

pub async fn verify_replay(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<ReplayVerification>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(replay_service) = &state.replay_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("replay service unavailable")),
        );
    };

    match replay_service.verify().await {
        Ok(result) => (StatusCode::OK, Json(ApiResponse::ok(result))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "failed to verify replay: {error}"
            ))),
        ),
    }
}

pub async fn replay_readiness(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<ReplayReadinessReport>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(replay_service) = &state.replay_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("replay service unavailable")),
        );
    };

    match replay_service.readiness().await {
        Ok(report) => (StatusCode::OK, Json(ApiResponse::ok(report))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "failed to build replay readiness report: {error}"
            ))),
        ),
    }
}

pub async fn export_audit_entries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditExportQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<AuditChainEntry>>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(audit_chain) = &state.audit_chain else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("audit chain unavailable")),
        );
    };

    match audit_chain
        .list_entries(
            query.limit.unwrap_or(200),
            query.before_seq,
            query.action.as_deref(),
            query.actor_id.as_deref(),
        )
        .await
    {
        Ok(entries) => (StatusCode::OK, Json(ApiResponse::ok(entries))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!(
                "failed to export audit entries: {error}"
            ))),
        ),
    }
}

pub async fn retention_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<RetentionPolicy>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    (
        StatusCode::OK,
        Json(ApiResponse::ok(state.retention_policy.clone())),
    )
}

pub async fn witness_topology(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<ApiResponse<WitnessTopology>>) {
    if let Err((status, message)) = require_permission(&state, &headers, Permission::Admin).await {
        return (status, Json(ApiResponse::err(message)));
    }

    let Some(service) = &state.witness_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::err("witness service unavailable")),
        );
    };

    (StatusCode::OK, Json(ApiResponse::ok(service.topology())))
}
