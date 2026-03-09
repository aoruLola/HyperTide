//! Storage Manager
//! Handles file upload/download operations with local and S3 backends

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    pub hash: String,
    pub original_path: String,
    pub size_bytes: u64,
    pub stored_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct StorageManager {
    storage_root: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager with the given root directory
    pub fn new(storage_root: impl AsRef<Path>) -> Self {
        Self {
            storage_root: storage_root.as_ref().to_path_buf(),
        }
    }

    /// Initialize storage directory structure
    pub async fn init(&self) -> Result<(), String> {
        // Create storage directories: objects/, temp/
        let objects_dir = self.storage_root.join("objects");
        let temp_dir = self.storage_root.join("temp");

        fs::create_dir_all(&objects_dir)
            .await
            .map_err(|e| format!("Failed to create objects dir: {}", e))?;
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        Ok(())
    }

    /// Calculate BLAKE3 hash of file content
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize().to_hex().to_string()
    }

    /// Store file content, returns the content hash
    /// Uses Content-Addressable Storage (CAS) - files stored by their hash
    pub async fn store(&self, data: &[u8], original_path: &str) -> Result<StoredFile, String> {
        let hash = Self::calculate_hash(data);
        let size_bytes = data.len() as u64;

        // CAS path: objects/ab/cdef1234... (first 2 chars as subdirectory)
        let (prefix, rest) = hash.split_at(2);
        let object_dir = self.storage_root.join("objects").join(prefix);
        let object_path = object_dir.join(rest);

        // Check if already exists (deduplication)
        if object_path.exists() {
            return Ok(StoredFile {
                hash,
                original_path: original_path.to_string(),
                size_bytes,
                stored_at: chrono::Utc::now(),
            });
        }

        // Create subdirectory if needed
        fs::create_dir_all(&object_dir)
            .await
            .map_err(|e| format!("Failed to create object subdir: {}", e))?;

        // Write file atomically (write to temp, then rename)
        let temp_path = self.storage_root.join("temp").join(&hash);
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        file.write_all(data)
            .await
            .map_err(|e| format!("Failed to write data: {}", e))?;

        file.sync_all()
            .await
            .map_err(|e| format!("Failed to sync file: {}", e))?;

        // Atomic rename. If another writer already won the race, treat as idempotent success.
        if let Err(rename_error) = fs::rename(&temp_path, &object_path).await {
            if object_path.exists() {
                let _ = fs::remove_file(&temp_path).await;
            } else {
                return Err(format!("Failed to move file to storage: {}", rename_error));
            }
        }

        Ok(StoredFile {
            hash,
            original_path: original_path.to_string(),
            size_bytes,
            stored_at: chrono::Utc::now(),
        })
    }

    /// Retrieve file content by hash
    pub async fn retrieve(&self, hash: &str) -> Result<Vec<u8>, String> {
        if hash.len() < 3 {
            return Err("Invalid hash".to_string());
        }

        let (prefix, rest) = hash.split_at(2);
        let object_path = self.storage_root.join("objects").join(prefix).join(rest);

        if !object_path.exists() {
            return Err(format!("Object not found: {}", hash));
        }

        fs::read(&object_path)
            .await
            .map_err(|e| format!("Failed to read object: {}", e))
    }

    /// Check if a file with given hash exists
    pub async fn exists(&self, hash: &str) -> bool {
        if hash.len() < 3 {
            return false;
        }

        let (prefix, rest) = hash.split_at(2);
        let object_path = self.storage_root.join("objects").join(prefix).join(rest);
        object_path.exists()
    }

    /// Get the local file path for a hash (for direct access)
    pub fn get_path(&self, hash: &str) -> Option<PathBuf> {
        if hash.len() < 3 {
            return None;
        }

        let (prefix, rest) = hash.split_at(2);
        Some(self.storage_root.join("objects").join(prefix).join(rest))
    }
}
