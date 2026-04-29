use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Clone)]
pub struct StatePaths {
    pub state_dir: PathBuf,
    pub profile_path: PathBuf,
    pub stage_path: PathBuf,
    pub workspace_path: PathBuf,
    pub cache_dir: PathBuf,
}

pub fn state_paths_from(base_dir: &Path) -> StatePaths {
    let state_dir = base_dir.join(".hypertide");
    StatePaths {
        profile_path: state_dir.join("profile.json"),
        stage_path: state_dir.join("stage.json"),
        workspace_path: state_dir.join("workspace.json"),
        cache_dir: state_dir.join("cache").join("objects"),
        state_dir,
    }
}

pub fn ensure_state_dirs(paths: &StatePaths) -> Result<()> {
    if !paths.state_dir.exists() {
        fs::create_dir_all(&paths.state_dir)?;
    }
    if !paths.cache_dir.exists() {
        fs::create_dir_all(&paths.cache_dir)?;
    }
    Ok(())
}

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub fn cache_object_path(paths: &StatePaths, hash: &str) -> PathBuf {
    paths.cache_dir.join(hash)
}
