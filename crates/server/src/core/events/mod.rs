use serde_json::Value;
use sqlx::PgPool;

#[derive(Clone)]
pub struct EventStore {
    pool: PgPool,
}

/// Attribution metadata for an event — who, with what tool, in which session.
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    pub workflow_id: Option<String>,
    pub tool_id: Option<String>,
    pub session_id: Option<String>,
}

impl EventMetadata {
    /// Extract attribution metadata from HTTP request headers.
    ///
    /// Reads `X-HT-Workflow-Id`, `X-HT-Tool-Id`, and `X-HT-Session-Id`.
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Self {
        Self {
            workflow_id: header_str(headers, "x-ht-workflow-id"),
            tool_id: header_str(headers, "x-ht-tool-id"),
            session_id: header_str(headers, "x-ht-session-id"),
        }
    }
}

fn header_str(headers: &axum::http::HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
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
        meta: &EventMetadata,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO event_store (event_type, actor_id, repo_id, changeset_id, payload, workflow_id, tool_id, session_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(event_type)
        .bind(actor_id)
        .bind(repo_id)
        .bind(changeset_id)
        .bind(payload)
        .bind(meta.workflow_id.as_deref())
        .bind(meta.tool_id.as_deref())
        .bind(meta.session_id.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
