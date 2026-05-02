use std::collections::HashSet;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;

use crate::api::{common::ApiResponse, middleware::authz};
use crate::core::{auth::Permission, storage::StorageManager};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestChunk {
    pub i: u32,
    pub chunk_hash: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateManifestRequest {
    pub version: u32,
    pub chunk_size_policy: String,
    pub chunks: Vec<ManifestChunk>,
    pub file_meta: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateManifestResponse {
    pub manifest_hash: String,
    pub merkle_root: String,
    pub chunk_count: usize,
    pub created: bool,
}

#[derive(Debug, Serialize)]
struct CanonicalManifest {
    version: u32,
    chunk_size_policy: String,
    chunks: Vec<ManifestChunk>,
    file_meta: Value,
}

async fn require_upload_permission(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    authz::require_permission(state, headers, Permission::Upload)
        .await
        .map(|_| ())
}

fn merkle_root(chunks: &[ManifestChunk]) -> String {
    if chunks.is_empty() {
        return StorageManager::calculate_hash(b"");
    }

    let mut level = chunks
        .iter()
        .map(|chunk| {
            StorageManager::calculate_hash(
                format!("{}:{}:{}", chunk.i, chunk.chunk_hash, chunk.size).as_bytes(),
            )
        })
        .collect::<Vec<_>>();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = pair.get(1).unwrap_or(&pair[0]);
            next.push(StorageManager::calculate_hash(
                format!("{left}{right}").as_bytes(),
            ));
        }
        level = next;
    }

    level
        .pop()
        .unwrap_or_else(|| StorageManager::calculate_hash(b""))
}

pub async fn create_manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut payload): Json<CreateManifestRequest>,
) -> (StatusCode, Json<ApiResponse<CreateManifestResponse>>) {
    if let Err((status, message)) = require_upload_permission(&state, &headers).await {
        return (status, Json(ApiResponse::err(message)));
    }

    if payload.chunks.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("chunks must not be empty")),
        );
    }
    if payload.chunk_size_policy.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("chunk_size_policy is required")),
        );
    }

    payload.chunks.sort_by_key(|chunk| chunk.i);
    for window in payload.chunks.windows(2) {
        if window[0].i == window[1].i {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::err("duplicate chunk index is not allowed")),
            );
        }
    }

    let chunk_hashes = payload
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_hash.clone())
        .collect::<Vec<_>>();

    let missing = if let Some(pool) = state.db_pool.as_ref() {
        match sqlx::query_scalar::<_, String>(
            r#"
            SELECT chunk_hash
            FROM chunks
            WHERE chunk_hash = ANY($1)
            "#,
        )
        .bind(&chunk_hashes)
        .fetch_all(pool)
        .await
        {
            Ok(existing) => {
                let existing_set: HashSet<String> = existing.into_iter().collect();
                chunk_hashes
                    .into_iter()
                    .filter(|hash| !existing_set.contains(hash))
                    .collect::<Vec<_>>()
            }
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::err(format!(
                        "failed to query chunk metadata: {error}"
                    ))),
                );
            }
        }
    } else {
        let mut missing = Vec::new();
        for hash in chunk_hashes {
            if !state.storage_manager.exists(&hash).await {
                missing.push(hash);
            }
        }
        missing
    };

    if !missing.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err(format!(
                "manifest references missing chunks: {}",
                missing.join(",")
            ))),
        );
    }

    let canonical = CanonicalManifest {
        version: payload.version,
        chunk_size_policy: payload.chunk_size_policy.clone(),
        chunks: payload.chunks.clone(),
        file_meta: payload.file_meta.unwrap_or(Value::Null),
    };
    let canonical_json = match serde_json::to_vec(&canonical) {
        Ok(data) => data,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::err(format!(
                    "failed to serialize canonical manifest: {error}"
                ))),
            );
        }
    };

    let manifest_hash = StorageManager::calculate_hash(&canonical_json);
    let merkle_root = merkle_root(&canonical.chunks);
    let mut created = true;

    if let Some(pool) = state.db_pool.as_ref() {
        match sqlx::query(
            r#"
            INSERT INTO manifests (
                manifest_hash,
                schema_version,
                chunk_size_policy,
                chunk_count,
                manifest_json,
                merkle_root
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (manifest_hash) DO NOTHING
            "#,
        )
        .bind(&manifest_hash)
        .bind(canonical.version as i32)
        .bind(&canonical.chunk_size_policy)
        .bind(canonical.chunks.len() as i32)
        .bind(serde_json::to_value(&canonical).unwrap_or(Value::Null))
        .bind(&merkle_root)
        .execute(pool)
        .await
        {
            Ok(result) => {
                created = result.rows_affected() > 0;
            }
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::err(format!(
                        "failed to persist manifest: {error}"
                    ))),
                );
            }
        }
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::ok({
            if let Some(event_store) = &state.event_store {
                if let Err(error) = event_store
                    .append(
                        "MANIFEST_CREATED",
                        "manifest-api",
                        None,
                        None,
                        json!({
                            "manifest_hash": manifest_hash,
                            "chunk_count": canonical.chunks.len(),
                            "chunk_size_policy": canonical.chunk_size_policy,
                            "created": created,
                        }),
                    )
                    .await
                {
                    tracing::warn!("failed to append manifest event: {error}");
                }
            }
            CreateManifestResponse {
                manifest_hash,
                merkle_root,
                chunk_count: canonical.chunks.len(),
                created,
            }
        })),
    )
}
