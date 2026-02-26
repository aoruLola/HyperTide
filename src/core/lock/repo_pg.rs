use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{FromRow, PgPool};

use crate::core::lock::FileLock;

#[derive(Clone)]
pub struct LockRepoPg {
    pool: PgPool,
}

#[derive(Debug, Deserialize, FromRow)]
struct LockRow {
    file_path: String,
    owner_id: String,
    locked_at: DateTime<Utc>,
    lease_expires_at: Option<DateTime<Utc>>,
}

impl LockRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_locks(&self) -> Result<Vec<FileLock>, sqlx::Error> {
        let rows = sqlx::query_as::<_, LockRow>(
            r#"
            SELECT file_path, owner_id, locked_at, lease_expires_at
            FROM locks
            WHERE force_released = FALSE
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| FileLock {
                file_path: row.file_path,
                owner_id: row.owner_id,
                locked_at: row.locked_at,
                lease_expires_at: row.lease_expires_at,
            })
            .collect())
    }

    pub async fn upsert_lock(&self, lock: &FileLock) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO locks (file_path, owner_id, locked_at, lease_expires_at, force_released)
            VALUES ($1, $2, $3, $4, FALSE)
            ON CONFLICT (file_path)
            DO UPDATE SET
                owner_id = EXCLUDED.owner_id,
                locked_at = EXCLUDED.locked_at,
                lease_expires_at = EXCLUDED.lease_expires_at,
                force_released = FALSE
            "#,
        )
        .bind(&lock.file_path)
        .bind(&lock.owner_id)
        .bind(lock.locked_at)
        .bind(lock.lease_expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_lock(&self, file_path: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM locks
            WHERE file_path = $1
            "#,
        )
        .bind(file_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
