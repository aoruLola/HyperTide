use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::common::ApiResponse;
use crate::core::auth::{ApiKey, Permission};
use crate::core::versioning::{
    AssetDelta, BranchRecord, ChangesetKind, ChangesetRecord, HistoryPage, SnapshotEntry,
    SubmitChangesetInput, SyncSnapshot, VersioningError, ROOT_BASE_CHANGESET_ID,
};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub repo_id: String,
    pub branch: String,
    pub from_changeset_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BranchListResponse {
    pub repo_id: String,
    pub branches: Vec<BranchRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitChangesetRequest {
    pub repo_id: String,
    pub branch: Option<String>,
    pub base_changeset_id: Option<String>,
    pub kind: Option<String>,
    pub rollback_of: Option<String>,
    pub author: String,
    pub message: String,
    pub assets: Vec<AssetDelta>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub branch: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub repo_id: String,
    pub branch: String,
    pub target_changeset_id: String,
    pub author: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RollbackResponse {
    pub rollback_plan: RollbackPlanView,
    pub changeset: ChangesetRecord,
}

#[derive(Debug, Serialize)]
pub struct RollbackPlanView {
    pub base_changeset_id: String,
    pub target_changeset_id: String,
    pub asset_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub branch: Option<String>,
    pub to_changeset_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub repo_id: String,
    pub branch: String,
    pub changeset_id: Option<String>,
    pub assets: Vec<SnapshotEntry>,
}

fn require_key(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<ApiKey, (StatusCode, String)> {
    let key = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let api_key = match state.auth_manager.validate_key(key) {
        Some(k) => k,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid API key".to_string())),
    };
    if !api_key.has_permission(permission) {
        return Err((StatusCode::FORBIDDEN, "Permission denied".to_string()));
    }
    Ok(api_key)
}

fn parse_kind(value: Option<&str>) -> Result<ChangesetKind, String> {
    match value.unwrap_or("normal").to_lowercase().as_str() {
        "normal" => Ok(ChangesetKind::Normal),
        "rollback" => Ok(ChangesetKind::Rollback),
        _ => Err("kind must be normal|rollback".to_string()),
    }
}

fn map_versioning_error(error: VersioningError) -> (StatusCode, String) {
    match error {
        VersioningError::RepoNotFound { repo_id } => {
            (StatusCode::NOT_FOUND, format!("Repo not found: {repo_id}"))
        }
        VersioningError::BranchNotFound { repo_id, branch } => (
            StatusCode::NOT_FOUND,
            format!("Branch not found: {repo_id}/{branch}"),
        ),
        VersioningError::BranchAlreadyExists { repo_id, branch } => (
            StatusCode::CONFLICT,
            format!("Branch already exists: {repo_id}/{branch}"),
        ),
        VersioningError::ChangesetNotFound {
            repo_id,
            changeset_id,
        } => (
            StatusCode::NOT_FOUND,
            format!("Changeset not found: {repo_id}/{changeset_id}"),
        ),
        VersioningError::BaseChangesetRequired => (
            StatusCode::BAD_REQUEST,
            format!("base_changeset_id is required. use {ROOT_BASE_CHANGESET_ID} for first commit"),
        ),
        VersioningError::BaseChangesetMismatch {
            repo_id,
            branch,
            expected,
            got,
        } => (
            StatusCode::CONFLICT,
            format!(
                "base_changeset_id mismatch for {repo_id}/{branch}, expected={expected:?}, got={got:?}"
            ),
        ),
        VersioningError::InvalidRollbackTarget {
            repo_id,
            branch,
            target_changeset_id,
        } => (
            StatusCode::CONFLICT,
            format!("Rollback target invalid: {repo_id}/{branch}/{target_changeset_id}"),
        ),
    }
}

fn ensure_lock_access(
    state: &AppState,
    owner_id: &str,
    assets: &[AssetDelta],
) -> Result<(), (StatusCode, String)> {
    for asset in assets {
        if let Some(lock) = state.lock_manager.get_lock(&asset.path) {
            if lock.owner_id != owner_id {
                return Err((
                    StatusCode::CONFLICT,
                    format!("Lock conflict on {}. owner={}", asset.path, lock.owner_id),
                ));
            }
        }
    }
    Ok(())
}

async fn ensure_blob_exists(
    state: &AppState,
    assets: &[AssetDelta],
) -> Result<(), (StatusCode, String)> {
    for asset in assets {
        if let Some(hash) = &asset.blob_hash {
            if !state.storage_manager.exists(hash).await {
                return Err((StatusCode::BAD_REQUEST, format!("Blob not found: {hash}")));
            }
        }
    }
    Ok(())
}

/// POST /v1/branches
pub async fn create_branch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateBranchRequest>,
) -> (StatusCode, Json<ApiResponse<BranchRecord>>) {
    let api_key = match require_key(&state, &headers, Permission::Upload) {
        Ok(key) => key,
        Err((status, message)) => return (status, Json(ApiResponse::err(message))),
    };

    match state.version_manager.create_branch(
        &payload.repo_id,
        &payload.branch,
        payload.from_changeset_id.as_deref(),
        &api_key.owner_id,
    ) {
        Ok(branch) => (StatusCode::CREATED, Json(ApiResponse::ok(branch))),
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            (status, Json(ApiResponse::err(message)))
        }
    }
}

/// GET /v1/branches/{repo_id}
pub async fn list_branches(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<BranchListResponse>>) {
    if let Err((status, message)) = require_key(&state, &headers, Permission::Download) {
        return (status, Json(ApiResponse::err(message)));
    }

    match state.version_manager.list_branches(&repo_id) {
        Ok(branches) => (
            StatusCode::OK,
            Json(ApiResponse::ok(BranchListResponse { repo_id, branches })),
        ),
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            (status, Json(ApiResponse::err(message)))
        }
    }
}

/// POST /v1/changesets
pub async fn submit_changeset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SubmitChangesetRequest>,
) -> (StatusCode, Json<ApiResponse<ChangesetRecord>>) {
    let api_key = match require_key(&state, &headers, Permission::Upload) {
        Ok(key) => key,
        Err((status, message)) => return (status, Json(ApiResponse::err(message))),
    };

    let kind = match parse_kind(payload.kind.as_deref()) {
        Ok(kind) => kind,
        Err(message) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::err(message))),
    };

    if payload.author != api_key.owner_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::err("author must match API key owner")),
        );
    }

    if let Err(err) = ensure_lock_access(&state, &api_key.owner_id, &payload.assets) {
        return (err.0, Json(ApiResponse::err(err.1)));
    }
    if let Err(err) = ensure_blob_exists(&state, &payload.assets).await {
        return (err.0, Json(ApiResponse::err(err.1)));
    }

    let input = SubmitChangesetInput {
        repo_id: payload.repo_id,
        branch: payload.branch.unwrap_or_else(|| "main".to_string()),
        base_changeset_id: payload.base_changeset_id,
        kind,
        rollback_of: payload.rollback_of,
        author: payload.author,
        message: payload.message,
        assets: payload.assets,
    };

    match state.version_manager.submit_changeset(input) {
        Ok(changeset) => (StatusCode::CREATED, Json(ApiResponse::ok(changeset))),
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            (status, Json(ApiResponse::err(message)))
        }
    }
}

/// GET /v1/history/{repo_id}?branch=...&limit=...&cursor=...
pub async fn list_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> (StatusCode, Json<ApiResponse<HistoryPage>>) {
    if let Err((status, message)) = require_key(&state, &headers, Permission::Download) {
        return (status, Json(ApiResponse::err(message)));
    }

    let branch = query.branch.unwrap_or_else(|| "main".to_string());
    let limit = query.limit.unwrap_or(20);
    let cursor = query.cursor.unwrap_or(0);
    match state
        .version_manager
        .history(&repo_id, &branch, limit, cursor)
    {
        Ok(page) => (StatusCode::OK, Json(ApiResponse::ok(page))),
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            (status, Json(ApiResponse::err(message)))
        }
    }
}

/// POST /v1/rollback
pub async fn rollback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RollbackRequest>,
) -> (StatusCode, Json<ApiResponse<RollbackResponse>>) {
    let api_key = match require_key(&state, &headers, Permission::Upload) {
        Ok(key) => key,
        Err((status, message)) => return (status, Json(ApiResponse::err(message))),
    };
    if payload.author != api_key.owner_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::err("author must match API key owner")),
        );
    }

    let plan = match state.version_manager.build_rollback_plan(
        &payload.repo_id,
        &payload.branch,
        &payload.target_changeset_id,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            return (status, Json(ApiResponse::err(message)));
        }
    };

    if let Err(err) = ensure_lock_access(&state, &api_key.owner_id, &plan.assets) {
        return (err.0, Json(ApiResponse::err(err.1)));
    }
    if let Err(err) = ensure_blob_exists(&state, &plan.assets).await {
        return (err.0, Json(ApiResponse::err(err.1)));
    }

    let message = payload
        .message
        .unwrap_or_else(|| format!("rollback: {}", payload.target_changeset_id));
    let input = SubmitChangesetInput {
        repo_id: plan.repo_id.clone(),
        branch: plan.branch.clone(),
        base_changeset_id: Some(plan.base_changeset_id.clone()),
        kind: ChangesetKind::Rollback,
        rollback_of: Some(plan.target_changeset_id.clone()),
        author: payload.author,
        message,
        assets: plan.assets.clone(),
    };

    match state.version_manager.submit_changeset(input) {
        Ok(changeset) => (
            StatusCode::CREATED,
            Json(ApiResponse::ok(RollbackResponse {
                rollback_plan: RollbackPlanView {
                    base_changeset_id: plan.base_changeset_id,
                    target_changeset_id: plan.target_changeset_id,
                    asset_count: plan.assets.len(),
                },
                changeset,
            })),
        ),
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            (status, Json(ApiResponse::err(message)))
        }
    }
}

/// GET /v1/sync/{repo_id}?branch=...&to_changeset_id=...
pub async fn sync_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<String>,
    Query(query): Query<SyncQuery>,
) -> (StatusCode, Json<ApiResponse<SyncResponse>>) {
    if let Err((status, message)) = require_key(&state, &headers, Permission::Download) {
        return (status, Json(ApiResponse::err(message)));
    }

    let branch = query.branch.unwrap_or_else(|| "main".to_string());
    let snapshot: SyncSnapshot = match state.version_manager.sync_snapshot(
        &repo_id,
        &branch,
        query.to_changeset_id.as_deref(),
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let (status, message) = map_versioning_error(error);
            return (status, Json(ApiResponse::err(message)));
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::ok(SyncResponse {
            repo_id: snapshot.repo_id,
            branch: snapshot.branch,
            changeset_id: snapshot.changeset_id,
            assets: snapshot.assets,
        })),
    )
}
