//! HyperTide VCS Server
//! High-performance asset version control for game development
//!
//! CARP Component - HyperTide CLI Surface 26.0.1 Preview

mod api;
mod core;

use axum::{
    extract::FromRef,
    routing::{get, post, delete},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::lock::{lock_file, unlock_file, force_unlock_file, list_locks};
use crate::api::storage::{upload_file, download_file, check_exists, calculate_hash};
use crate::api::auth::{verify_key, generate_key, revoke_key, list_keys};
use crate::core::lock::LockManager;
use crate::core::storage::StorageManager;
use crate::core::auth::AuthManager;

const VERSION: &str = "Surface 26.0.1 Preview";

/// 开发模式 Master Key (仅用于开发/测试)
const DEV_MASTER_KEY: &str = "dev-master-key";

fn print_banner() {
    // ANSI color codes - Red to Purple gradient
    const RED: &str = "\x1b[91m";      // Bright red
    const MAGENTA: &str = "\x1b[95m";  // Bright magenta/purple
    const PURPLE: &str = "\x1b[35m";   // Purple
    const RESET: &str = "\x1b[0m";
    const DIM: &str = "\x1b[2m";
    const GREEN: &str = "\x1b[92m";

    println!();
    println!("{DIM}══════════════════════════════════════════════════════════════════════{RESET}");
    println!();
    println!("  {DIM}CARP                                              Designed by Lyura{RESET}");
    println!();
    println!("{RED}  ██╗  ██╗██╗   ██╗██████╗ ███████╗██████╗ ████████╗██╗██████╗ ███████╗{RESET}");
    println!("{RED}  ██║  ██║╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗╚══██╔══╝██║██╔══██╗██╔════╝{RESET}");
    println!("{MAGENTA}  ███████║ ╚████╔╝ ██████╔╝█████╗  ██████╔╝   ██║   ██║██║  ██║█████╗  {RESET}");
    println!("{MAGENTA}  ██╔══██║  ╚██╔╝  ██╔═══╝ ██╔══╝  ██╔══██╗   ██║   ██║██║  ██║██╔══╝  {RESET}");
    println!("{PURPLE}  ██║  ██║   ██║   ██║     ███████╗██║  ██║   ██║   ██║██████╔╝███████╗{RESET}");
    println!("{PURPLE}  ╚═╝  ╚═╝   ╚═╝   ╚═╝     ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚═╝╚═════╝ ╚══════╝{RESET}");
    println!();
    println!("  {DIM}CLI Surface 26.0.1 Preview{RESET}");
    println!();
    println!("{DIM}══════════════════════════════════════════════════════════════════════{RESET}");
    println!();
    println!("  {GREEN}🔑 Dev Master Key: {DEV_MASTER_KEY}{RESET}");
    println!();
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub lock_manager: LockManager,
    pub storage_manager: StorageManager,
    pub auth_manager: AuthManager,
}

// Allow extracting LockManager from AppState
impl FromRef<AppState> for LockManager {
    fn from_ref(state: &AppState) -> Self {
        state.lock_manager.clone()
    }
}

// Allow extracting StorageManager from AppState
impl FromRef<AppState> for StorageManager {
    fn from_ref(state: &AppState) -> Self {
        state.storage_manager.clone()
    }
}

// Allow extracting AuthManager from AppState
impl FromRef<AppState> for AuthManager {
    fn from_ref(state: &AppState) -> Self {
        state.auth_manager.clone()
    }
}

#[tokio::main]
async fn main() {
    // Print ASCII banner
    print_banner();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "hypertide=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize storage
    let storage_manager = StorageManager::new("./storage");
    if let Err(e) = storage_manager.init().await {
        tracing::error!("Failed to initialize storage: {}", e);
        return;
    }
    tracing::info!("📦 Storage initialized at ./storage");

    // Initialize authentication (with dev master key)
    let auth_manager = AuthManager::with_dev_key(DEV_MASTER_KEY);
    tracing::info!("🔑 Auth initialized (dev mode)");

    // Initialize shared state
    let lock_manager = LockManager::new();
    
    let state = AppState {
        lock_manager,
        storage_manager,
        auth_manager,
    };

    // CORS configuration (allow all for development)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Define routes
    let app = Router::new()
        // Health & Info
        .route("/", get(root))
        .route("/health", get(health))
        // Lock API
        .route("/api/lock", post(lock_file))
        .route("/api/unlock", delete(unlock_file))
        .route("/api/break-lock", post(force_unlock_file))
        .route("/api/locks", get(list_locks))
        // Storage API
        .route("/api/upload", post(upload_file))
        .route("/api/download/{hash}", get(download_file))
        .route("/api/exists/{hash}", get(check_exists))
        .route("/api/hash", post(calculate_hash))
        // Auth API
        .route("/api/auth/verify", get(verify_key))
        .route("/api/auth/generate", post(generate_key))
        .route("/api/auth/revoke", delete(revoke_key))
        .route("/api/auth/keys", get(list_keys))
        // Middleware
        .layer(cors)
        // Shared State (unified)
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🌊 {} - Listening on http://{}", VERSION, addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Root endpoint - Version info
async fn root() -> &'static str {
    "🌊 HyperTide CLI Surface 26.0.1 Preview (Running)"
}

/// Health check for Docker/K8s
async fn health() -> &'static str {
    "OK"
}

