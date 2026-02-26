use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::core::checkpoint::CheckpointRecord;

#[derive(Debug, Clone)]
struct WitnessKey {
    id: String,
    secret: String,
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
}

#[derive(Clone)]
pub struct WitnessService {
    pool: PgPool,
    witnesses: Vec<WitnessKey>,
    quorum: usize,
}

impl WitnessService {
    pub fn from_env(pool: PgPool) -> Self {
        // format: "w1:secret1,w2:secret2,w3:secret3"
        let configured = std::env::var("WITNESS_KEYS").unwrap_or_else(|_| {
            "witness-a:dev-secret-a,witness-b:dev-secret-b,witness-c:dev-secret-c".to_string()
        });
        let mut witnesses = configured
            .split(',')
            .filter_map(|item| {
                let mut parts = item.splitn(2, ':');
                let id = parts.next()?.trim();
                let secret = parts.next()?.trim();
                if id.is_empty() || secret.is_empty() {
                    return None;
                }
                Some(WitnessKey {
                    id: id.to_string(),
                    secret: secret.to_string(),
                })
            })
            .collect::<Vec<_>>();
        if witnesses.is_empty() {
            witnesses = vec![
                WitnessKey {
                    id: "witness-a".to_string(),
                    secret: "dev-secret-a".to_string(),
                },
                WitnessKey {
                    id: "witness-b".to_string(),
                    secret: "dev-secret-b".to_string(),
                },
                WitnessKey {
                    id: "witness-c".to_string(),
                    secret: "dev-secret-c".to_string(),
                },
            ];
        }

        let quorum = std::env::var("WITNESS_QUORUM")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(2)
            .clamp(1, witnesses.len());

        Self {
            pool,
            witnesses,
            quorum,
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

        Ok(WitnessSummary {
            checkpoint_id: checkpoint_id.to_string(),
            quorum: self.quorum,
            quorum_met: receipts.len() >= self.quorum,
            receipts,
        })
    }
}
