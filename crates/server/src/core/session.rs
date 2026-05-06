use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionRecord {
    pub session_id: String,
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub workspace_root: String,
    pub actor_id: String,
    pub intent_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub trigger_reason: Option<String>,
    pub risk_level: Option<String>,
    pub semantic_summary: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub workspace_root: String,
    pub actor_id: String,
    pub intent_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub trigger_reason: Option<String>,
    pub risk_level: Option<String>,
    pub semantic_summary: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointAsset {
    pub asset_id: String,
    pub path: String,
    pub blob_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCheckpointRecord {
    pub checkpoint_id: String,
    pub session_id: String,
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub parent_checkpoint_id: Option<String>,
    pub workspace_root: String,
    pub actor_id: String,
    pub trigger_reason: String,
    pub intent_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub risk_level: Option<String>,
    pub semantic_summary: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub assets: Vec<CheckpointAsset>,
}

#[derive(Debug, Clone)]
pub struct CreateCheckpointInput {
    pub session_id: String,
    pub actor_id: String,
    pub trigger_reason: String,
    pub semantic_summary: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub assets: Vec<CheckpointAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckpointPage {
    pub items: Vec<SessionCheckpointRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckpointSnapshot {
    pub checkpoint_id: String,
    pub session_id: String,
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub workspace_root: String,
    pub assets: Vec<CheckpointAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    SessionNotFound { session_id: String },
    CheckpointNotFound { checkpoint_id: String },
    CheckpointExpired { checkpoint_id: String },
}

#[derive(Debug, Default)]
struct SessionState {
    sessions: HashMap<String, AgentSessionRecord>,
    checkpoints: HashMap<String, SessionCheckpointRecord>,
    session_checkpoint_ids: HashMap<String, Vec<String>>,
}

#[derive(Clone, Default)]
pub struct SessionManager {
    state: Arc<RwLock<SessionState>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_records(
        sessions: Vec<AgentSessionRecord>,
        checkpoints: Vec<SessionCheckpointRecord>,
    ) -> Self {
        let manager = Self::new();
        {
            let mut state = manager.state.write().expect("session lock poisoned");
            for session in sessions {
                state
                    .session_checkpoint_ids
                    .entry(session.session_id.clone())
                    .or_default();
                state.sessions.insert(session.session_id.clone(), session);
            }
            let mut checkpoints = checkpoints;
            checkpoints.sort_by_key(|checkpoint| checkpoint.created_at);
            for checkpoint in checkpoints {
                state
                    .session_checkpoint_ids
                    .entry(checkpoint.session_id.clone())
                    .or_default()
                    .push(checkpoint.checkpoint_id.clone());
                state
                    .checkpoints
                    .insert(checkpoint.checkpoint_id.clone(), checkpoint);
            }
        }
        manager
    }

    pub async fn with_pg(pool: PgPool) -> Result<Self, sqlx::Error> {
        let sessions = sqlx::query_as::<_, AgentSessionRow>(
            r#"
            SELECT session_id, repo_id, branch_name, base_changeset_id, workspace_root, actor_id,
                   intent_id, task_id, agent_run_id, trigger_reason, risk_level, semantic_summary,
                   expires_at, created_at
            FROM agent_sessions
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(AgentSessionRecord::from)
        .collect::<Vec<_>>();

        let checkpoint_rows = sqlx::query_as::<_, SessionCheckpointRow>(
            r#"
            SELECT checkpoint_id, session_id, repo_id, branch_name, base_changeset_id,
                   parent_checkpoint_id, workspace_root, actor_id, trigger_reason, intent_id,
                   task_id, agent_run_id, risk_level, semantic_summary, expires_at, created_at
            FROM session_checkpoints
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&pool)
        .await?;

        let asset_rows = sqlx::query_as::<_, CheckpointAssetRow>(
            r#"
            SELECT checkpoint_id, asset_id, path, blob_hash
            FROM checkpoint_assets
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&pool)
        .await?;
        let mut assets_by_checkpoint: HashMap<String, Vec<CheckpointAsset>> = HashMap::new();
        for row in asset_rows {
            assets_by_checkpoint
                .entry(row.checkpoint_id)
                .or_default()
                .push(CheckpointAsset {
                    asset_id: row.asset_id,
                    path: row.path,
                    blob_hash: row.blob_hash,
                });
        }

        let checkpoints = checkpoint_rows
            .into_iter()
            .map(|row| {
                let checkpoint_id = row.checkpoint_id.clone();
                SessionCheckpointRecord {
                    checkpoint_id: checkpoint_id.clone(),
                    session_id: row.session_id,
                    repo_id: row.repo_id,
                    branch: row.branch_name,
                    base_changeset_id: row.base_changeset_id,
                    parent_checkpoint_id: row.parent_checkpoint_id,
                    workspace_root: row.workspace_root,
                    actor_id: row.actor_id,
                    trigger_reason: row.trigger_reason,
                    intent_id: row.intent_id,
                    task_id: row.task_id,
                    agent_run_id: row.agent_run_id,
                    risk_level: row.risk_level,
                    semantic_summary: row.semantic_summary,
                    expires_at: row.expires_at,
                    created_at: row.created_at,
                    assets: assets_by_checkpoint
                        .remove(&checkpoint_id)
                        .unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>();

        Ok(Self::from_records(sessions, checkpoints))
    }

    pub fn create_session(&self, input: CreateSessionInput) -> AgentSessionRecord {
        let record = AgentSessionRecord {
            session_id: Uuid::new_v4().to_string(),
            repo_id: input.repo_id,
            branch: input.branch,
            base_changeset_id: input.base_changeset_id,
            workspace_root: input.workspace_root,
            actor_id: input.actor_id,
            intent_id: input.intent_id,
            task_id: input.task_id,
            agent_run_id: input.agent_run_id,
            trigger_reason: input.trigger_reason,
            risk_level: input.risk_level,
            semantic_summary: input.semantic_summary,
            expires_at: input.expires_at,
            created_at: Utc::now(),
        };
        let mut state = self.state.write().expect("session lock poisoned");
        state
            .sessions
            .insert(record.session_id.clone(), record.clone());
        state
            .session_checkpoint_ids
            .entry(record.session_id.clone())
            .or_default();
        record
    }

    pub fn create_checkpoint(
        &self,
        input: CreateCheckpointInput,
    ) -> Result<SessionCheckpointRecord, SessionError> {
        let mut state = self.state.write().expect("session lock poisoned");
        let session = state
            .sessions
            .get(&input.session_id)
            .cloned()
            .ok_or_else(|| SessionError::SessionNotFound {
                session_id: input.session_id.clone(),
            })?;
        let parent_checkpoint_id = state
            .session_checkpoint_ids
            .get(&input.session_id)
            .and_then(|ids| ids.last())
            .cloned();
        let record = SessionCheckpointRecord {
            checkpoint_id: Uuid::new_v4().to_string(),
            session_id: input.session_id,
            repo_id: session.repo_id,
            branch: session.branch,
            base_changeset_id: session.base_changeset_id,
            parent_checkpoint_id,
            workspace_root: session.workspace_root,
            actor_id: input.actor_id,
            trigger_reason: input.trigger_reason,
            intent_id: session.intent_id,
            task_id: session.task_id,
            agent_run_id: session.agent_run_id,
            risk_level: session.risk_level,
            semantic_summary: input.semantic_summary.or(session.semantic_summary),
            expires_at: input.expires_at.or(session.expires_at),
            created_at: Utc::now(),
            assets: input.assets,
        };
        state
            .session_checkpoint_ids
            .entry(record.session_id.clone())
            .or_default()
            .push(record.checkpoint_id.clone());
        state
            .checkpoints
            .insert(record.checkpoint_id.clone(), record.clone());
        Ok(record)
    }

    pub fn list_checkpoints(&self, session_id: &str) -> Result<CheckpointPage, SessionError> {
        let state = self.state.read().expect("session lock poisoned");
        if !state.sessions.contains_key(session_id) {
            return Err(SessionError::SessionNotFound {
                session_id: session_id.to_string(),
            });
        }
        let items = state
            .session_checkpoint_ids
            .get(session_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| state.checkpoints.get(id).cloned())
            .collect();
        Ok(CheckpointPage { items })
    }

    pub fn checkpoint_snapshot(
        &self,
        checkpoint_id: &str,
    ) -> Result<CheckpointSnapshot, SessionError> {
        let state = self.state.read().expect("session lock poisoned");
        let checkpoint = state.checkpoints.get(checkpoint_id).ok_or_else(|| {
            SessionError::CheckpointNotFound {
                checkpoint_id: checkpoint_id.to_string(),
            }
        })?;
        if checkpoint
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
        {
            return Err(SessionError::CheckpointExpired {
                checkpoint_id: checkpoint_id.to_string(),
            });
        }
        Ok(CheckpointSnapshot {
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            session_id: checkpoint.session_id.clone(),
            repo_id: checkpoint.repo_id.clone(),
            branch: checkpoint.branch.clone(),
            base_changeset_id: checkpoint.base_changeset_id.clone(),
            workspace_root: checkpoint.workspace_root.clone(),
            assets: checkpoint.assets.clone(),
        })
    }
}

#[derive(Debug, FromRow)]
struct AgentSessionRow {
    session_id: String,
    repo_id: String,
    branch_name: String,
    base_changeset_id: Option<String>,
    workspace_root: String,
    actor_id: String,
    intent_id: Option<String>,
    task_id: Option<String>,
    agent_run_id: Option<String>,
    trigger_reason: Option<String>,
    risk_level: Option<String>,
    semantic_summary: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<AgentSessionRow> for AgentSessionRecord {
    fn from(row: AgentSessionRow) -> Self {
        Self {
            session_id: row.session_id,
            repo_id: row.repo_id,
            branch: row.branch_name,
            base_changeset_id: row.base_changeset_id,
            workspace_root: row.workspace_root,
            actor_id: row.actor_id,
            intent_id: row.intent_id,
            task_id: row.task_id,
            agent_run_id: row.agent_run_id,
            trigger_reason: row.trigger_reason,
            risk_level: row.risk_level,
            semantic_summary: row.semantic_summary,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct SessionCheckpointRow {
    checkpoint_id: String,
    session_id: String,
    repo_id: String,
    branch_name: String,
    base_changeset_id: Option<String>,
    parent_checkpoint_id: Option<String>,
    workspace_root: String,
    actor_id: String,
    trigger_reason: String,
    intent_id: Option<String>,
    task_id: Option<String>,
    agent_run_id: Option<String>,
    risk_level: Option<String>,
    semantic_summary: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct CheckpointAssetRow {
    checkpoint_id: String,
    asset_id: String,
    path: String,
    blob_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_checkpoint_and_lists_it_for_session() {
        let manager = SessionManager::new();
        let session = manager.create_session(CreateSessionInput {
            repo_id: "repo-agent".to_string(),
            branch: "main".to_string(),
            base_changeset_id: Some("ROOT".to_string()),
            workspace_root: "E:/workspace/game".to_string(),
            actor_id: "agent-a".to_string(),
            intent_id: Some("intent-1".to_string()),
            task_id: Some("task-1".to_string()),
            agent_run_id: Some("run-1".to_string()),
            trigger_reason: Some("agent_save".to_string()),
            risk_level: Some("local".to_string()),
            semantic_summary: Some("saving agent progress".to_string()),
            expires_at: None,
        });

        let checkpoint = manager
            .create_checkpoint(CreateCheckpointInput {
                session_id: session.session_id.clone(),
                actor_id: "agent-a".to_string(),
                trigger_reason: "manual_checkpoint".to_string(),
                semantic_summary: Some("inventory draft checkpoint".to_string()),
                expires_at: None,
                assets: vec![CheckpointAsset {
                    asset_id: "asset-inventory".to_string(),
                    path: "Assets/inventory.json".to_string(),
                    blob_hash: "hash-inventory".to_string(),
                }],
            })
            .expect("checkpoint");

        assert_eq!(checkpoint.parent_checkpoint_id, None);
        assert_eq!(checkpoint.base_changeset_id.as_deref(), Some("ROOT"));

        let page = manager
            .list_checkpoints(&session.session_id)
            .expect("checkpoint page");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].checkpoint_id, checkpoint.checkpoint_id);

        let snapshot = manager
            .checkpoint_snapshot(&checkpoint.checkpoint_id)
            .expect("snapshot");
        assert_eq!(snapshot.assets.len(), 1);
        assert_eq!(snapshot.assets[0].path, "Assets/inventory.json");
    }

    #[test]
    fn rebuilds_session_state_from_persisted_records() {
        let created_at = Utc::now();
        let session = AgentSessionRecord {
            session_id: "session-1".to_string(),
            repo_id: "repo-agent".to_string(),
            branch: "main".to_string(),
            base_changeset_id: Some("ROOT".to_string()),
            workspace_root: "E:/workspace/game".to_string(),
            actor_id: "agent-a".to_string(),
            intent_id: None,
            task_id: None,
            agent_run_id: None,
            trigger_reason: Some("agent_session".to_string()),
            risk_level: None,
            semantic_summary: None,
            expires_at: None,
            created_at,
        };
        let checkpoint = SessionCheckpointRecord {
            checkpoint_id: "checkpoint-1".to_string(),
            session_id: session.session_id.clone(),
            repo_id: session.repo_id.clone(),
            branch: session.branch.clone(),
            base_changeset_id: session.base_changeset_id.clone(),
            parent_checkpoint_id: None,
            workspace_root: session.workspace_root.clone(),
            actor_id: session.actor_id.clone(),
            trigger_reason: "manual_checkpoint".to_string(),
            intent_id: None,
            task_id: None,
            agent_run_id: None,
            risk_level: None,
            semantic_summary: None,
            expires_at: None,
            created_at,
            assets: vec![CheckpointAsset {
                asset_id: "asset-a".to_string(),
                path: "Assets/a.txt".to_string(),
                blob_hash: "hash-a".to_string(),
            }],
        };

        let manager = SessionManager::from_records(vec![session], vec![checkpoint]);
        let snapshot = manager
            .checkpoint_snapshot("checkpoint-1")
            .expect("checkpoint must survive manager rebuild");

        assert_eq!(snapshot.session_id, "session-1");
        assert_eq!(snapshot.assets[0].blob_hash, "hash-a");
    }
}
