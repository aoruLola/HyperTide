//! HyperTide backend server

mod api;
mod core;

use axum::{
    extract::{DefaultBodyLimit, FromRef, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::auth::{
    exchange_key, generate_key, list_keys, refresh_token, revoke_key, revoke_refresh_token,
    verify_key,
};
use crate::api::blobs::{missing_chunks, upload_chunk};
use crate::api::lock::{force_unlock_file, list_locks, lock_file, renew_lock_file, unlock_file};
use crate::api::manifests::create_manifest;
use crate::api::storage::{calculate_hash, check_exists, download_file, upload_file};
use crate::api::trust::{
    attest_checkpoint, export_audit_entries, generate_checkpoint, latest_checkpoint,
    replay_readiness, retention_policy, verify_audit_chain, verify_replay, witness_summary,
    witness_topology,
};
use crate::api::versioning::{
    approve_changeset, changeset_gate, create_branch, list_branches, list_history,
    promote_changeset, rollback, submit_changeset, sync_snapshot,
};
use crate::core::audit_chain::AuditChain;
use crate::core::auth::AuthManager;
use crate::core::checkpoint::CheckpointService;
use crate::core::compliance::RetentionPolicy;
use crate::core::config::{AppConfig, AppEnv};
use crate::core::db::{migrations::run_migrations, pool::init_pg_pool_from_env};
use crate::core::events::EventStore;
use crate::core::high_risk::HighRiskGuard;
use crate::core::lock::LockManager;
use crate::core::replay::ReplayService;
use crate::core::storage::StorageManager;
use crate::core::versioning::VersionManager;
use crate::core::witness::WitnessService;

const VERSION: &str = "Surface 26.0.1 Preview";
const DEFAULT_BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const UPLOAD_BODY_LIMIT_BYTES: usize = 256 * 1024 * 1024;

fn print_banner(env: AppEnv) {
    println!();
    println!("==============================================================");
    println!(" HyperTide Backend {}", VERSION);
    println!(" Environment: {}", env.as_str());
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
    pub audit_chain: Option<AuditChain>,
    pub checkpoint_service: Option<CheckpointService>,
    pub witness_service: Option<WitnessService>,
    pub high_risk_guard: Option<HighRiskGuard>,
    pub replay_service: Option<ReplayService>,
    pub retention_policy: RetentionPolicy,
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

fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    if config.app_env.is_production() {
        CorsLayer::new()
            .allow_origin(config.cors_allowed_origins.clone())
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}

fn build_app(state: AppState, config: &AppConfig) -> Router {
    let cors = build_cors_layer(config);

    let general_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health_live))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/v2/locks/acquire", post(lock_file))
        .route("/v2/locks/release", post(unlock_file))
        .route("/v2/locks/renew", post(renew_lock_file))
        .route("/v2/locks/force-release", post(force_unlock_file))
        .route("/v2/locks", get(list_locks))
        .route("/v2/storage/download/:hash", get(download_file))
        .route("/v2/storage/exists/:hash", get(check_exists))
        .route("/v2/storage/hash", post(calculate_hash))
        .route("/v2/blobs/missing", post(missing_chunks))
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
        .route(
            "/v2/changesets/:changeset_id/approve",
            post(approve_changeset),
        )
        .route(
            "/v2/changesets/:changeset_id/promote",
            post(promote_changeset),
        )
        .route("/v2/changesets/:changeset_id/gate", get(changeset_gate))
        .route("/v2/history/:repo_id", get(list_history))
        .route("/v2/rollback", post(rollback))
        .route("/v2/sync/:repo_id", get(sync_snapshot))
        .route("/v2/trust/checkpoints/generate", post(generate_checkpoint))
        .route("/v2/trust/checkpoints/latest", get(latest_checkpoint))
        .route(
            "/v2/trust/checkpoints/:checkpoint_id/witness/attest",
            post(attest_checkpoint),
        )
        .route("/v2/trust/witness/summary", get(witness_summary))
        .route("/v2/trust/witness/topology", get(witness_topology))
        .route("/v2/trust/audit/verify", post(verify_audit_chain))
        .route("/v2/trust/audit/export", get(export_audit_entries))
        .route("/v2/trust/retention/policy", get(retention_policy))
        .route("/v2/trust/replay/verify", post(verify_replay))
        .route("/v2/trust/replay/readiness", get(replay_readiness))
        .layer(DefaultBodyLimit::max(DEFAULT_BODY_LIMIT_BYTES))
        .layer(RequestBodyLimitLayer::new(DEFAULT_BODY_LIMIT_BYTES));

    let upload_routes = Router::new()
        .route("/v2/storage/upload", post(upload_file))
        .route("/v2/blobs/chunks/:chunk_hash", put(upload_chunk))
        .layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT_BYTES))
        .layer(RequestBodyLimitLayer::new(UPLOAD_BODY_LIMIT_BYTES));

    Router::new()
        .merge(general_routes)
        .merge(upload_routes)
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Configuration error: {error}");
            return;
        }
    };
    print_banner(config.app_env);

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

    let storage_manager = StorageManager::new(&config.storage_path);
    if let Err(e) = storage_manager.init().await {
        tracing::error!("Failed to initialize storage: {}", e);
        return;
    }
    tracing::info!("Storage initialized at {}", &config.storage_path);

    let auth_manager =
        match AuthManager::with_dev_key_and_db(config.master_key.clone(), db_pool.clone()).await {
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
        audit_chain: Some(AuditChain::new(db_pool.clone())),
        checkpoint_service: Some(CheckpointService::new(db_pool.clone())),
        witness_service: Some(WitnessService::from_env(db_pool.clone())),
        high_risk_guard: Some(HighRiskGuard::from_env(db_pool.clone())),
        replay_service: Some(ReplayService::new(db_pool.clone())),
        retention_policy: RetentionPolicy::from_env(),
        db_pool: Some(db_pool),
    };

    let app = build_app(state, &config);

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

    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "READY"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "DB_UNAVAILABLE"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{HeaderValue, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::util::ServiceExt;

    fn test_master_key() -> &'static str {
        "dev-master-key"
    }

    fn test_config() -> AppConfig {
        AppConfig {
            app_env: AppEnv::Development,
            master_key: test_master_key().to_string(),
            storage_path: "./storage".to_string(),
            cors_allowed_origins: Vec::<HeaderValue>::new(),
        }
    }

    fn test_state() -> AppState {
        AppState {
            lock_manager: LockManager::new(),
            storage_manager: StorageManager::new("./storage"),
            auth_manager: AuthManager::with_dev_key(test_master_key()),
            version_manager: VersionManager::new(),
            event_store: None,
            audit_chain: None,
            checkpoint_service: None,
            witness_service: None,
            high_risk_guard: None,
            replay_service: None,
            retention_policy: RetentionPolicy::from_env(),
            db_pool: None,
        }
    }

    #[tokio::test]
    async fn exists_route_returns_json_response() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/storage/exists/abcdef")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload["success"], Value::Bool(true));
    }

    #[tokio::test]
    async fn lock_route_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

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
    async fn lock_route_rejects_spoofed_owner_id() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .method("POST")
            .uri("/v2/locks/acquire")
            .header("content-type", "application/json")
            .header("X-API-Key", test_master_key())
            .body(Body::from(
                r#"{"file_path":"assets/a.txt","owner_id":"spoofed-user"}"#,
            ))
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn lock_route_uses_authenticated_owner_when_owner_not_provided() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .method("POST")
            .uri("/v2/locks/acquire")
            .header("content-type", "application/json")
            .header("X-API-Key", test_master_key())
            .body(Body::from(r#"{"file_path":"assets/no-owner.txt"}"#))
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            payload["data"]["owner_id"],
            Value::String("dev-admin".to_string())
        );
    }

    #[tokio::test]
    async fn non_upload_routes_reject_oversized_request_body() {
        let app = build_app(test_state(), &test_config());
        let oversized_data = "A".repeat(DEFAULT_BODY_LIMIT_BYTES + 1024);
        let payload = format!(r#"{{"data":"{oversized_data}"}}"#);

        let request = Request::builder()
            .method("POST")
            .uri("/v2/storage/hash")
            .header("content-type", "application/json")
            .header("X-API-Key", test_master_key())
            .body(Body::from(payload))
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn upload_routes_allow_request_body_larger_than_default_limit() {
        let app = build_app(test_state(), &test_config());
        let body = vec![b'x'; DEFAULT_BODY_LIMIT_BYTES + 1024];

        let request = Request::builder()
            .method("PUT")
            .uri("/v2/blobs/chunks/not-a-valid-hash")
            .header("X-API-Key", test_master_key())
            .body(Body::from(body))
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn ready_returns_503_without_db_pool() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/health/ready")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn replay_verify_returns_503_when_service_unavailable() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .method("POST")
            .uri("/v2/trust/replay/verify")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn replay_verify_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .method("POST")
            .uri("/v2/trust/replay/verify")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn replay_readiness_returns_503_when_service_unavailable() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/replay/readiness")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn replay_readiness_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/replay/readiness")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn changeset_gate_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/changesets/test-id/gate?repo_id=repo-a")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn changeset_gate_returns_not_found_for_unknown_repo() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/changesets/test-id/gate?repo_id=repo-a")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn audit_export_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/audit/export")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn audit_export_returns_503_when_service_unavailable() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/audit/export")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn retention_policy_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/retention/policy")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn retention_policy_returns_ok_with_admin_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/retention/policy")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn witness_topology_rejects_missing_api_key() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/witness/topology")
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn witness_topology_returns_503_when_service_unavailable() {
        let app = build_app(test_state(), &test_config());

        let request = Request::builder()
            .uri("/v2/trust/witness/topology")
            .header("X-API-Key", test_master_key())
            .body(Body::empty())
            .expect("request");

        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
