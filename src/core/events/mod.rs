use serde_json::Value;
use sqlx::PgPool;

#[derive(Clone)]
pub struct EventStore {
    pool: PgPool,
}

impl EventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn append(
        &self,
        event_type: &str,
        actor_id: &str,
        repo_id: Option<&str>,
        changeset_id: Option<&str>,
        payload: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO event_store (event_type, actor_id, repo_id, changeset_id, payload)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(event_type)
        .bind(actor_id)
        .bind(repo_id)
        .bind(changeset_id)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
