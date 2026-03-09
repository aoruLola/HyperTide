use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn init_pg_pool_from_env() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL is required for startup (postgres connection string)")?;
    let max_connections = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);
    let acquire_timeout_secs = std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
        .connect(&database_url)
        .await
        .context("failed to connect to postgres")
}
