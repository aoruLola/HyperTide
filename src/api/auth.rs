//! Authentication API Handlers
//! HTTP endpoints for API key management and authentication

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::api::common::ApiResponse;
use crate::core::auth::{AuthManager, Permission};

// ==================== Request/Response DTOs ====================

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub owner_id: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateKeyRequest {
    pub owner_id: String,
    pub permissions: Vec<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct GenerateKeyResponse {
    pub key: String,
    pub owner_id: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeKeyRequest {
    pub key: String,
}

// ==================== Helper Functions ====================

fn parse_permission(s: &str) -> Option<Permission> {
    match s.to_lowercase().as_str() {
        "lock" => Some(Permission::Lock),
        "upload" => Some(Permission::Upload),
        "download" => Some(Permission::Download),
        "admin" => Some(Permission::Admin),
        _ => None,
    }
}

fn permission_to_string(p: &Permission) -> String {
    match p {
        Permission::Lock => "lock".to_string(),
        Permission::Upload => "upload".to_string(),
        Permission::Download => "download".to_string(),
        Permission::Admin => "admin".to_string(),
    }
}

// ==================== API Handlers ====================

/// GET /api/auth/verify
/// 验证当前请求的 API Key 是否有效
/// API Key 从 X-API-Key header 获取 (由中间件注入)
pub async fn verify_key(
    State(manager): State<AuthManager>,
    headers: axum::http::HeaderMap,
) -> Json<ApiResponse<VerifyResponse>> {
    let key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match manager.validate_key(key) {
        Some(api_key) => Json(ApiResponse::ok(VerifyResponse {
            valid: true,
            owner_id: Some(api_key.owner_id),
            permissions: api_key
                .permissions
                .iter()
                .map(permission_to_string)
                .collect(),
        })),
        None => Json(ApiResponse::ok(VerifyResponse {
            valid: false,
            owner_id: None,
            permissions: vec![],
        })),
    }
}

/// POST /api/auth/generate
/// 生成新的 API Key (需要 Admin 权限)
pub async fn generate_key(
    State(manager): State<AuthManager>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<GenerateKeyRequest>,
) -> (StatusCode, Json<ApiResponse<GenerateKeyResponse>>) {
    // 验证调用者权限
    let caller_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !manager.has_permission(caller_key, Permission::Admin) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::err("Admin permission required")),
        );
    }

    // 解析权限
    let permissions: Vec<Permission> = payload
        .permissions
        .iter()
        .filter_map(|s| parse_permission(s))
        .collect();

    if permissions.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("At least one valid permission required")),
        );
    }

    // 生成 Key
    let api_key = manager.generate_key(&payload.owner_id, permissions, payload.expires_in_days);

    (
        StatusCode::CREATED,
        Json(ApiResponse::ok(GenerateKeyResponse {
            key: api_key.key,
            owner_id: api_key.owner_id,
            permissions: api_key
                .permissions
                .iter()
                .map(permission_to_string)
                .collect(),
            expires_at: api_key.expires_at.map(|dt| dt.to_rfc3339()),
        })),
    )
}

/// DELETE /api/auth/revoke
/// 撤销 API Key (需要 Admin 权限)
pub async fn revoke_key(
    State(manager): State<AuthManager>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RevokeKeyRequest>,
) -> (StatusCode, Json<ApiResponse<bool>>) {
    // 验证调用者权限
    let caller_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !manager.has_permission(caller_key, Permission::Admin) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::err("Admin permission required")),
        );
    }

    // 防止撤销自己
    if caller_key == payload.key {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::err("Cannot revoke your own key")),
        );
    }

    // 撤销 Key
    let revoked = manager.revoke_key(&payload.key);
    if revoked {
        (StatusCode::OK, Json(ApiResponse::ok(true)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::err("Key not found")),
        )
    }
}

/// GET /api/auth/keys
/// 列出所有 API Keys (需要 Admin 权限，隐藏完整 Key)
#[derive(Debug, Serialize)]
pub struct KeyListItem {
    pub key_prefix: String, // 只显示前缀
    pub owner_id: String,
    pub permissions: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked: bool,
}

pub async fn list_keys(
    State(manager): State<AuthManager>,
    headers: axum::http::HeaderMap,
) -> (StatusCode, Json<ApiResponse<Vec<KeyListItem>>>) {
    // 验证调用者权限
    let caller_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !manager.has_permission(caller_key, Permission::Admin) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::err("Admin permission required")),
        );
    }

    let keys: Vec<KeyListItem> = manager
        .list_keys()
        .into_iter()
        .map(|k| KeyListItem {
            // 只显示 Key 前 12 个字符
            key_prefix: format!("{}...", &k.key[..k.key.len().min(12)]),
            owner_id: k.owner_id,
            permissions: k.permissions.iter().map(permission_to_string).collect(),
            created_at: k.created_at.to_rfc3339(),
            expires_at: k.expires_at.map(|dt| dt.to_rfc3339()),
            revoked: k.revoked,
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::ok(keys)))
}
