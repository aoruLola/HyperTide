use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashSet;

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

#[derive(Debug, sqlx::FromRow)]
struct EventRow {
    event_type: String,
    payload: Value,
    changeset_id: Option<String>,
}

#[derive(Debug, Default)]
struct ReplayAccumulator {
    summary: ReplaySummary,
    current_locks: HashSet<String>,
    current_visible_changesets: HashSet<String>,
}

impl ReplayAccumulator {
    fn apply_event(&mut self, event_type: &str, payload: Option<&Value>, changeset_id: Option<&str>) {
        self.summary.events_processed += 1;
        match event_type {
            "LOCK_ACQUIRED" => {
                self.summary.lock_acquired += 1;
                if let Some(path) = extract_file_path(payload) {
                    self.current_locks.insert(path.to_string());
                }
            }
            "LOCK_RELEASED" | "LOCK_FORCE_RELEASED" => {
                self.summary.lock_released += 1;
                if let Some(path) = extract_file_path(payload) {
                    self.current_locks.remove(path);
                }
            }
            "CHANGESET_VISIBLE" | "ROLLBACK_VISIBLE" => {
                self.summary.changeset_visible += 1;
                if let Some(id) = changeset_id {
                    self.current_visible_changesets.insert(id.to_string());
                }
            }
            "CHANGESET_APPROVED" => {
                self.summary.changeset_approved += 1;
            }
            "CHANGESET_PROMOTED" => {
                self.summary.changeset_promoted += 1;
                if let Some(id) = changeset_id {
                    self.current_visible_changesets.insert(id.to_string());
                }
            }
            _ => {}
        }
    }
}

fn extract_file_path(payload: Option<&Value>) -> Option<&str> {
    payload?.get("file_path")?.as_str()
}

impl ReplayService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn verify(&self) -> Result<ReplayVerification, sqlx::Error> {
        let events = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT event_type, payload, changeset_id
            FROM event_store
            ORDER BY event_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut replay = ReplayAccumulator::default();
        for event in &events {
            replay.apply_event(
                &event.event_type,
                Some(&event.payload),
                event.changeset_id.as_deref(),
            );
        }

        let current_locks = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM locks WHERE force_released = FALSE",
        )
        .fetch_one(&self.pool)
        .await?;
        let current_visible_changesets =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM changesets WHERE status = 'visible'")
                .fetch_one(&self.pool)
                .await?;

        let mut mismatches = Vec::new();
        if replay.current_locks.len() as i64 != current_locks {
            mismatches.push(format!(
                "locks mismatch: replay_current={}, current={}",
                replay.current_locks.len(),
                current_locks
            ));
        }
        if replay.current_visible_changesets.len() as i64 != current_visible_changesets {
            mismatches.push(format!(
                "visible changesets mismatch: replay_current={}, current_visible_changesets={}",
                replay.current_visible_changesets.len(),
                current_visible_changesets
            ));
        }

        Ok(ReplayVerification {
            summary: replay.summary,
            current_locks,
            current_visible_changesets,
            consistency_ok: mismatches.is_empty(),
            mismatches,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayAccumulator;
    use serde_json::json;

    #[test]
    fn rebuilds_lock_state_from_event_payloads() {
        let mut acc = ReplayAccumulator::default();

        acc.apply_event("LOCK_ACQUIRED", Some(&json!({ "file_path": "assets/a.uasset" })), None);
        acc.apply_event("LOCK_RENEWED", Some(&json!({ "file_path": "assets/a.uasset" })), None);
        acc.apply_event("LOCK_RELEASED", Some(&json!({ "file_path": "assets/a.uasset" })), None);

        assert_eq!(acc.current_locks.len(), 0);
        assert_eq!(acc.summary.lock_acquired, 1);
        assert_eq!(acc.summary.lock_released, 1);
    }

    #[test]
    fn rebuilds_visible_changeset_state_from_events() {
        let mut acc = ReplayAccumulator::default();

        acc.apply_event("CHANGESET_VISIBLE", Some(&json!({})), Some("cs_visible"));
        acc.apply_event("CHANGESET_APPROVED", Some(&json!({})), Some("cs_draft"));
        acc.apply_event("CHANGESET_PROMOTED", Some(&json!({})), Some("cs_promoted"));
        acc.apply_event("ROLLBACK_VISIBLE", Some(&json!({})), Some("cs_rollback"));

        assert_eq!(acc.current_visible_changesets.len(), 3);
        assert_eq!(acc.summary.changeset_visible, 2);
        assert_eq!(acc.summary.changeset_approved, 1);
        assert_eq!(acc.summary.changeset_promoted, 1);
    }
}
