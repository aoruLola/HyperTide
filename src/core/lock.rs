use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx::PgPool;

pub mod repo_pg;
use self::repo_pg::LockRepoPg;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLock {
    pub file_path: String,
    pub owner_id: String,
    pub locked_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct LockManager {
    // Key: file_path, Value: Lock Info
    // DashMap provides high-concurrency access without heavy Mutex contention
    locks: Arc<DashMap<String, FileLock>>,
    repo: Option<LockRepoPg>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            repo: None,
        }
    }

    pub async fn with_pg(pool: PgPool) -> Result<Self, String> {
        let repo = LockRepoPg::new(pool);
        let manager = Self {
            locks: Arc::new(DashMap::new()),
            repo: Some(repo.clone()),
        };

        let existing = repo
            .load_locks()
            .await
            .map_err(|e| format!("failed to load locks from db: {e}"))?;
        for lock in existing {
            manager.locks.insert(lock.file_path.clone(), lock);
        }

        Ok(manager)
    }

    /// Attempt to lock a file. Returns true if successful, false if already locked by someone else.
    pub async fn try_lock(&self, file_path: String, owner_id: String) -> Result<FileLock, String> {
        // Atomic check and insert
        if let Some(existing) = self.locks.get(&file_path) {
            if existing.owner_id != owner_id {
                return Err(format!("File is already locked by {}", existing.owner_id));
            }
            // Idempotent: if already locked by me, return success
            return Ok(existing.clone());
        }

        let lock = FileLock {
            file_path: file_path.clone(),
            owner_id,
            locked_at: Utc::now(),
        };

        if let Some(repo) = &self.repo {
            repo.upsert_lock(&lock)
                .await
                .map_err(|e| format!("failed to persist lock: {e}"))?;
        }

        self.locks.insert(file_path, lock.clone());
        Ok(lock)
    }

    /// Unlock a file. Only the owner can unlock.
    pub async fn unlock(&self, file_path: &str, owner_id: &str) -> Result<(), String> {
        // We need to check ownership before removing
        if let Some(existing) = self.locks.get(file_path) {
            if existing.owner_id != owner_id {
                return Err(format!(
                    "Cannot unlock: File is locked by {}",
                    existing.owner_id
                ));
            }
        } else {
            return Err("File is not locked".to_string());
        }

        if let Some(repo) = &self.repo {
            repo.delete_lock(file_path)
                .await
                .map_err(|e| format!("failed to delete lock: {e}"))?;
        }

        self.locks.remove(file_path);
        Ok(())
    }

    /// Admin force unlock
    pub async fn force_unlock(&self, file_path: &str) -> Result<bool, String> {
        if let Some(repo) = &self.repo {
            repo.delete_lock(file_path)
                .await
                .map_err(|e| format!("failed to force release lock: {e}"))?;
        }
        Ok(self.locks.remove(file_path).is_some())
    }

    /// List all locks (for administrative view or debugging)
    pub fn list_locks(&self) -> Vec<FileLock> {
        self.locks.iter().map(|kv| kv.value().clone()).collect()
    }

    /// Query lock by path.
    pub fn get_lock(&self, file_path: &str) -> Option<FileLock> {
        self.locks.get(file_path).map(|entry| entry.clone())
    }
}
