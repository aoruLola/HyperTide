use axum::{extract::State, http::StatusCode};

use crate::AppState;

pub(crate) async fn root() -> &'static str {
    "HyperTide Backend running (v2)"
}

pub(crate) async fn health_live() -> &'static str {
    "OK"
}

pub(crate) async fn health_ready(State(state): State<AppState>) -> (StatusCode, &'static str) {
    let Some(pool) = state.db_pool.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "DB_NOT_CONFIGURED");
    };

    if let Err(error) = state.storage_manager.health_check().await {
        tracing::warn!(error = %error, "Storage readiness check failed");
        return (StatusCode::SERVICE_UNAVAILABLE, "STORAGE_UNAVAILABLE");
    }

    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "READY"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "DB_UNAVAILABLE"),
    }
}
