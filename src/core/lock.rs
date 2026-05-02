use chrono::{DateTime, Utc};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

pub mod repo_pg;
use self::repo_pg::LockRepoPg;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLock {
    pub file_path: String,
    pub owner_id: String,
    pub locked_at: DateTime<Utc>,
    pub lease_expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct LockManager {
    // Key: file_path, Value: Lock Info
    // DashMap provides high-concurrency access without heavy Mutex contention
    locks: Arc<DashMap<String, FileLock>>,
    repo: Option<LockRepoPg>,
    lease_seconds: i64,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            repo: None,
            lease_seconds: default_lease_seconds(),
        }
    }

    pub async fn with_pg(pool: PgPool) -> Result<Self, String> {
        let repo = LockRepoPg::new(pool);
        let manager = Self {
            locks: Arc::new(DashMap::new()),
            repo: Some(repo.clone()),
            lease_seconds: default_lease_seconds(),
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
        let requested_lock = FileLock {
            file_path: file_path.clone(),
            owner_id: owner_id.clone(),
            locked_at: Utc::now(),
            lease_expires_at: Some(self.next_lease_expiry()),
        };

        if let Some(repo) = &self.repo {
            let effective_lock = repo
                .acquire_lock_atomic(&requested_lock)
                .await
                .map_err(|e| format!("failed to persist lock: {e}"))?;
            self.locks
                .insert(effective_lock.file_path.clone(), effective_lock.clone());
            if effective_lock.owner_id != owner_id {
                return Err(format!(
                    "File is already locked by {}",
                    effective_lock.owner_id
                ));
            }
            return Ok(effective_lock);
        }

        match self.locks.entry(file_path.clone()) {
            Entry::Occupied(mut occupied) => {
                let existing = occupied.get().clone();
                if self.is_expired(&existing) {
                    occupied.insert(requested_lock.clone());
                    Ok(requested_lock)
                } else if existing.owner_id != owner_id {
                    Err(format!("File is already locked by {}", existing.owner_id))
                } else {
                    Ok(existing)
                }
            }
            Entry::Vacant(vacant) => {
                vacant.insert(requested_lock.clone());
                Ok(requested_lock)
            }
        }
    }

    pub async fn renew_lock(&self, file_path: &str, owner_id: &str) -> Result<FileLock, String> {
        let existing = self
            .locks
            .get(file_path)
            .map(|entry| entry.clone())
            .ok_or_else(|| "File is not locked".to_string())?;

        if existing.owner_id != owner_id {
            return Err(format!(
                "Cannot renew: File is locked by {}",
                existing.owner_id
            ));
        }
        if self.is_expired(&existing) {
            if let Some(repo) = &self.repo {
                repo.delete_lock(file_path)
                    .await
                    .map_err(|e| format!("failed to cleanup expired lock: {e}"))?;
            }
            self.locks.remove(file_path);
            return Err("Cannot renew: lock lease expired".to_string());
        }

        let renewed = FileLock {
            lease_expires_at: Some(self.next_lease_expiry()),
            ..existing
        };

        if let Some(repo) = &self.repo {
            repo.upsert_lock(&renewed)
                .await
                .map_err(|e| format!("failed to persist lock renew: {e}"))?;
        }
        self.locks.insert(file_path.to_string(), renewed.clone());
        Ok(renewed)
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
        self.locks
            .iter()
            .map(|kv| kv.value().clone())
            .filter(|lock| !self.is_expired(lock))
            .collect()
    }

    /// Query lock by path.
    pub fn get_lock(&self, file_path: &str) -> Option<FileLock> {
        self.locks
            .get(file_path)
            .map(|entry| entry.clone())
            .filter(|lock| !self.is_expired(lock))
    }

    fn next_lease_expiry(&self) -> DateTime<Utc> {
        Utc::now() + chrono::Duration::seconds(self.lease_seconds.max(30))
    }

    fn is_expired(&self, lock: &FileLock) -> bool {
        lock.lease_expires_at
            .map(|expiry| expiry <= Utc::now())
            .unwrap_or(false)
    }
}

fn default_lease_seconds() -> i64 {
    std::env::var("LOCK_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(300)
}
