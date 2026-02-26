//! HyperTide backend server

mod api;
mod core;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::auth::{
    exchange_key, generate_key, list_keys, refresh_token, revoke_key, revoke_refresh_token,
    verify_key,
};
use crate::api::blobs::{missing_chunks, upload_chunk};
use crate::api::lock::{force_unlock_file, list_locks, lock_file, renew_lock_file, unlock_file};
use crate::api::manifests::create_manifest;
use crate::api::storage::{calculate_hash, check_exists, download_file, upload_file};
use crate::api::versioning::{
    approve_changeset, create_branch, list_branches, list_history, promote_changeset, rollback,
    submit_changeset, sync_snapshot,
};
use crate::core::auth::AuthManager;
use crate::core::db::{migrations::run_migrations, pool::init_pg_pool_from_env};
use crate::core::events::EventStore;
use crate::core::lock::LockManager;
use crate::core::storage::StorageManager;
use crate::core::versioning::VersionManager;

const VERSION: &str = "Surface 26.0.1 Preview";
const DEV_MASTER_KEY: &str = "dev-master-key";

fn print_banner() {
    println!();
    println!("==============================================================");
    println!(" HyperTide Backend {}", VERSION);
    println!(" Dev Master Key: {}", DEV_MASTER_KEY);
    println!("==============================================================");
    println!();
}

#[derive(Clone)]
pub struct AppState {
    pub lock_manager: LockManager,
    pub storage_manager: StorageManager,
    pub auth_manager: AuthManager,
    pub version_manager: VersionManager,
    pub event_store: Option<EventStore>,
    pub db_pool: Option<PgPool>,
}

impl FromRef<AppState> for LockManager {
    fn from_ref(state: &AppState) -> Self {
        state.lock_manager.clone()
    }
}

impl FromRef<AppState> for StorageManager {
    fn from_ref(state: &AppState) -> Self {
        state.storage_manager.clone()
    }
}

impl FromRef<AppState> for AuthManager {
    fn from_ref(state: &AppState) -> Self {
        state.auth_manager.clone()
    }
}

impl FromRef<AppState> for VersionManager {
    fn from_ref(state: &AppState) -> Self {
        state.version_manager.clone()
    }
}

fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(root))
        .route("/health", get(health_live))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v2/locks/acquire", post(lock_file))
        .route("/v2/locks/release", post(unlock_file))
        .route("/v2/locks/renew", post(renew_lock_file))
        .route("/v2/locks/force-release", post(force_unlock_file))
        .route("/v2/locks", get(list_locks))
        .route("/v2/storage/upload", post(upload_file))
        .route("/v2/storage/download/:hash", get(download_file))
        .route("/v2/storage/exists/:hash", get(check_exists))
        .route("/v2/storage/hash", post(calculate_hash))
        .route("/v2/blobs/missing", post(missing_chunks))
        .route("/v2/blobs/chunks/:chunk_hash", put(upload_chunk))
        .route("/v2/manifests", post(create_manifest))
        .route("/v2/auth/verify", get(verify_key))
        .route("/v2/auth/generate", post(generate_key))
        .route("/v2/auth/revoke", delete(revoke_key))
        .route("/v2/auth/keys", get(list_keys))
        .route("/v2/auth/exchange-key", post(exchange_key))
        .route("/v2/auth/refresh", post(refresh_token))
        .route("/v2/auth/revoke-refresh", post(revoke_refresh_token))
        .route("/v2/branches", post(create_branch))
        .route("/v2/branches/:repo_id", get(list_branches))
        .route("/v2/changesets", post(submit_changeset))
        .route("/v2/changesets/:changeset_id/approve", post(approve_changeset))
        .route("/v2/changesets/:changeset_id/promote", post(promote_changeset))
        .route("/v2/history/:repo_id", get(list_history))
        .route("/v2/rollback", post(rollback))
        .route("/v2/sync/:repo_id", get(sync_snapshot))
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    print_banner();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "hypertide_cli=info,hypertide=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_pool = match init_pg_pool_from_env().await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to initialize postgres pool: {e}");
            return;
        }
    };

    if let Err(e) = run_migrations(&db_pool).await {
        tracing::error!("Failed to run database migrations: {e}");
        return;
    }
    tracing::info!("Database ready (pool + migrations)");

    let storage_manager = StorageManager::new("./storage");
    if let Err(e) = storage_manager.init().await {
        tracing::error!("Failed to initialize storage: {}", e);
        return;
    }
    tracing::info!("Storage initialized at ./storage");

    let auth_manager =
        match AuthManager::with_dev_key_and_db(DEV_MASTER_KEY, db_pool.clone()).await {
            Ok(manager) => manager,
            Err(e) => {
                tracing::error!("Failed to initialize auth manager: {e}");
                return;
            }
        };
    let lock_manager = match LockManager::with_pg(db_pool.clone()).await {
        Ok(manager) => manager,
        Err(e) => {
            tracing::error!("Failed to initialize lock manager: {e}");
            return;
        }
    };

    let version_manager = match VersionManager::with_pg(db_pool.clone()).await {
        Ok(manager) => manager,
        Err(e) => {
            tracing::error!("Failed to initialize version manager: {e}");
            return;
        }
    };

    let state = AppState {
        lock_manager,
        storage_manager,
        auth_manager,
        version_manager,
        event_store: Some(EventStore::new(db_pool.clone())),
        db_pool: Some(db_pool),
    };

    let app = build_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("{} - listening on http://{}", VERSION, addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("Failed to bind address: {e}");
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server exited with error: {e}");
    }
}

async fn root() -> &'static str {
    "HyperTide Backend running (v2)"
}

async fn health_live() -> &'static str {
    "OK"
}

async fn health_ready(State(state): State<AppState>) -> (StatusCode, &'static str) {
    let Some(pool) = state.db_pool.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "DB_NOT_CONFIGURED");
    };

    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pool).await {
        Ok(_) => (StatusCode::OK, "READY"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "DB_UNAVAILABLE"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::util::ServiceExt;

    fn test_state() -> AppState {
        AppState {
            lock_manager: LockManager::new(),
            storage_manager: StorageManager::new("./storage"),
            auth_manager: AuthManager::with_dev_key(DEV_MASTER_KEY),
            version_manager: VersionManager::new(),
            event_store: None,
            db_pool: None,
        }
    }

    #[tokio::test]
    async fn exists_route_returns_json_response() {
        let app = build_app(test_state());

        let request = Request::builder()
            .uri("/v2/storage/exists/abcdef")
            .header("X-API-Key", DEV_MASTER_KEY)
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload["success"], Value::Bool(true));
    }

    #[tokio::test]
    async fn lock_route_rejects_missing_api_key() {
        let app = build_app(test_state());

        let request = Request::builder()
            .method("POST")
            .uri("/v2/locks/acquire")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"file_path":"assets/a.txt","owner_id":"alice"}"#,
            ))
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn ready_returns_503_without_db_pool() {
        let app = build_app(test_state());

        let request = Request::builder()
            .uri("/health/ready")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
