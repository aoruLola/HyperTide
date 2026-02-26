use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AuditChain {
    pool: PgPool,
}

impl AuditChain {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn append(
        &self,
        action: &str,
        actor_id: &str,
        repo_id: Option<&str>,
        target_id: Option<&str>,
        payload: Value,
    ) -> Result<String, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // serialize append writes to avoid hash forks under concurrency
        sqlx::query("SELECT pg_advisory_xact_lock(92426001)")
            .execute(&mut *tx)
            .await?;

        let prev_hash = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT entry_hash
            FROM audit_chain_entries
            ORDER BY seq DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or_else(|| "GENESIS".to_string());

        let created_at = Utc::now();
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let payload_hash = blake3::hash(&payload_bytes).to_hex().to_string();
        let material = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            prev_hash,
            action,
            actor_id,
            repo_id.unwrap_or_default(),
            target_id.unwrap_or_default(),
            payload_hash,
            created_at.timestamp_millis()
        );
        let entry_hash = blake3::hash(material.as_bytes()).to_hex().to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_chain_entries (
                prev_hash, entry_hash, action, actor_id, repo_id, target_id, payload, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&prev_hash)
        .bind(&entry_hash)
        .bind(action)
        .bind(actor_id)
        .bind(repo_id)
        .bind(target_id)
        .bind(payload)
        .bind(created_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(entry_hash)
    }
}
