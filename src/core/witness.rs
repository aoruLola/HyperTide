use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use std::collections::HashSet;

use crate::core::checkpoint::CheckpointRecord;

#[derive(Debug, Clone)]
struct WitnessKey {
    id: String,
    secret: String,
    scope: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct WitnessReceipt {
    pub checkpoint_id: String,
    pub witness_id: String,
    pub signature: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WitnessSummary {
    pub checkpoint_id: String,
    pub receipts: Vec<WitnessReceipt>,
    pub quorum: usize,
    pub quorum_met: bool,
    pub distinct_scopes: Vec<String>,
    pub cross_scope_quorum_met: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WitnessScopeEntry {
    pub witness_id: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WitnessTopology {
    pub scope: String,
    pub witness_ids: Vec<String>,
    pub witness_scopes: Vec<WitnessScopeEntry>,
    pub quorum: usize,
}

#[derive(Clone)]
pub struct WitnessService {
    pool: PgPool,
    witnesses: Vec<WitnessKey>,
    quorum: usize,
    scope: String,
}

impl WitnessService {
    pub fn from_env(pool: PgPool) -> Self {
        // format: "w1:secret1:scope1,w2:secret2:scope2"
        let configured = std::env::var("WITNESS_KEYS").unwrap_or_else(|_| {
            "witness-a:dev-secret-a,witness-b:dev-secret-b,witness-c:dev-secret-c".to_string()
        });
        let default_scope =
            std::env::var("WITNESS_DEFAULT_SCOPE").unwrap_or_else(|_| "local".to_string());
        let mut witnesses = configured
            .split(',')
            .filter_map(|item| {
                let mut parts = item.splitn(3, ':');
                let id = parts.next()?.trim();
                let secret = parts.next()?.trim();
                let scope = parts
                    .next()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| default_scope.clone());
                if id.is_empty() || secret.is_empty() {
                    return None;
                }
                Some(WitnessKey {
                    id: id.to_string(),
                    secret: secret.to_string(),
                    scope,
                })
            })
            .collect::<Vec<_>>();
        if witnesses.is_empty() {
            witnesses = vec![
                WitnessKey {
                    id: "witness-a".to_string(),
                    secret: "dev-secret-a".to_string(),
                    scope: default_scope.clone(),
                },
                WitnessKey {
                    id: "witness-b".to_string(),
                    secret: "dev-secret-b".to_string(),
                    scope: default_scope.clone(),
                },
                WitnessKey {
                    id: "witness-c".to_string(),
                    secret: "dev-secret-c".to_string(),
                    scope: default_scope,
                },
            ];
        }

        let quorum = std::env::var("WITNESS_QUORUM")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(2)
            .clamp(1, witnesses.len());
        let scope =
            std::env::var("WITNESS_SCOPE").unwrap_or_else(|_| "single-env".to_string());

        Self {
            pool,
            witnesses,
            quorum,
            scope,
        }
    }

    pub async fn attest(
        &self,
        checkpoint_id: &str,
        witness_id: Option<&str>,
    ) -> Result<WitnessReceipt, String> {
        let checkpoint = sqlx::query_as::<_, CheckpointRecord>(
            r#"
            SELECT checkpoint_id, log_head_hash, log_size, state_root, created_at
            FROM trust_checkpoints
            WHERE checkpoint_id = $1
            "#,
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| format!("failed to query checkpoint: {error}"))?
        .ok_or_else(|| "checkpoint not found".to_string())?;

        let witness = match witness_id {
            Some(id) => self
                .witnesses
                .iter()
                .find(|w| w.id == id)
                .ok_or_else(|| "witness_id not configured".to_string())?,
            None => self
                .witnesses
                .first()
                .ok_or_else(|| "no witness configured".to_string())?,
        };

        let material = format!(
            "{}|{}|{}|{}",
            checkpoint.checkpoint_id,
            checkpoint.log_head_hash,
            checkpoint.log_size,
            checkpoint.state_root
        );
        let signature = blake3::hash(format!("{}|{}", witness.secret, material).as_bytes())
            .to_hex()
            .to_string();
        let created_at = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO witness_receipts (checkpoint_id, witness_id, signature, created_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (checkpoint_id, witness_id)
            DO UPDATE SET signature = EXCLUDED.signature, created_at = EXCLUDED.created_at
            "#,
        )
        .bind(checkpoint_id)
        .bind(&witness.id)
        .bind(&signature)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(|error| format!("failed to persist witness receipt: {error}"))?;

        Ok(WitnessReceipt {
            checkpoint_id: checkpoint_id.to_string(),
            witness_id: witness.id.clone(),
            signature,
            created_at,
        })
    }

    pub async fn summary(&self, checkpoint_id: &str) -> Result<WitnessSummary, String> {
        let receipts = sqlx::query_as::<_, WitnessReceipt>(
            r#"
            SELECT checkpoint_id, witness_id, signature, created_at
            FROM witness_receipts
            WHERE checkpoint_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(checkpoint_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("failed to query witness receipts: {error}"))?;

        let mut scopes = HashSet::new();
        for receipt in &receipts {
            if let Some(scope) = self
                .witnesses
                .iter()
                .find(|w| w.id == receipt.witness_id)
                .map(|w| w.scope.clone())
            {
                scopes.insert(scope);
            }
        }
        let mut distinct_scopes = scopes.into_iter().collect::<Vec<_>>();
        distinct_scopes.sort();

        Ok(WitnessSummary {
            checkpoint_id: checkpoint_id.to_string(),
            quorum: self.quorum,
            quorum_met: receipts.len() >= self.quorum,
            cross_scope_quorum_met: receipts.len() >= self.quorum && distinct_scopes.len() >= 2,
            distinct_scopes,
            receipts,
        })
    }

    pub fn topology(&self) -> WitnessTopology {
        WitnessTopology {
            scope: self.scope.clone(),
            witness_ids: self.witnesses.iter().map(|w| w.id.clone()).collect(),
            witness_scopes: self
                .witnesses
                .iter()
                .map(|w| WitnessScopeEntry {
                    witness_id: w.id.clone(),
                    scope: w.scope.clone(),
                })
                .collect(),
            quorum: self.quorum,
        }
    }
}
