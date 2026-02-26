use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReplaySummary {
    pub events_processed: i64,
    pub lock_acquired: i64,
    pub lock_released: i64,
    pub changeset_visible: i64,
    pub changeset_approved: i64,
    pub changeset_promoted: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayVerification {
    pub summary: ReplaySummary,
    pub current_locks: i64,
    pub current_visible_changesets: i64,
    pub consistency_ok: bool,
    pub mismatches: Vec<String>,
}

#[derive(Clone)]
pub struct ReplayService {
    pool: PgPool,
}

impl ReplayService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn verify(&self) -> Result<ReplayVerification, sqlx::Error> {
        let event_types = sqlx::query_scalar::<_, String>(
            r#"
            SELECT event_type
            FROM event_store
            ORDER BY event_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summary = ReplaySummary::default();
        for event_type in &event_types {
            summary.events_processed += 1;
            match event_type.as_str() {
                "LOCK_ACQUIRED" => summary.lock_acquired += 1,
                "LOCK_RELEASED" | "LOCK_FORCE_RELEASED" => summary.lock_released += 1,
                "CHANGESET_VISIBLE" | "ROLLBACK_VISIBLE" => summary.changeset_visible += 1,
                "CHANGESET_APPROVED" => summary.changeset_approved += 1,
                "CHANGESET_PROMOTED" => summary.changeset_promoted += 1,
                _ => {}
            }
        }

        let current_locks =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM locks WHERE force_released = FALSE")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
        let current_visible_changesets =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM changesets WHERE status = 'visible'")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let replay_estimated_locks = summary.lock_acquired.saturating_sub(summary.lock_released);
        let mut mismatches = Vec::new();
        if replay_estimated_locks != current_locks {
            mismatches.push(format!(
                "locks mismatch: replay_estimated={}, current={}",
                replay_estimated_locks, current_locks
            ));
        }
        if summary.changeset_visible < current_visible_changesets {
            mismatches.push(format!(
                "visible changesets mismatch: replay_visible_events={}, current_visible_changesets={}",
                summary.changeset_visible, current_visible_changesets
            ));
        }

        Ok(ReplayVerification {
            summary,
            current_locks,
            current_visible_changesets,
            consistency_ok: mismatches.is_empty(),
            mismatches,
        })
    }
}
