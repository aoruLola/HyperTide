use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
        }
    }

    /// Attempt to lock a file. Returns true if successful, false if already locked by someone else.
    pub fn try_lock(&self, file_path: String, owner_id: String) -> Result<FileLock, String> {
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

        self.locks.insert(file_path, lock.clone());
        Ok(lock)
    }

    /// Unlock a file. Only the owner can unlock.
    pub fn unlock(&self, file_path: &str, owner_id: &str) -> Result<(), String> {
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

        self.locks.remove(file_path);
        Ok(())
    }

    /// Admin force unlock
    pub fn force_unlock(&self, file_path: &str) -> bool {
        self.locks.remove(file_path).is_some()
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
