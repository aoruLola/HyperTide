use axum::http::{HeaderMap, StatusCode};

use crate::{
    core::auth::{AuthIdentity, Permission},
    AppState,
};

pub async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<AuthIdentity, (StatusCode, String)> {
    require_permission_with_options(state, headers, permission, true).await
}

pub async fn require_permission_with_options(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
    allow_api_key: bool,
) -> Result<AuthIdentity, (StatusCode, String)> {
    let identity = if let Some(bearer) = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_bearer_token)
    {
        state
            .auth_manager
            .validate_access_token(bearer)
            .await
            .map_err(|message| (StatusCode::UNAUTHORIZED, message))?
    } else if allow_api_key {
        let api_key = headers
            .get("X-API-Key")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if api_key.is_empty() {
            return Err((StatusCode::UNAUTHORIZED, "Missing credentials".to_string()));
        }
        state
            .auth_manager
            .validate_api_key_identity(api_key)
            .await
            .map_err(|message| (StatusCode::UNAUTHORIZED, message))?
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Missing bearer token".to_string()));
    };

    if !identity.has_permission(permission) {
        return Err((StatusCode::FORBIDDEN, "Permission denied".to_string()));
    }

    Ok(identity)
}

fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}
