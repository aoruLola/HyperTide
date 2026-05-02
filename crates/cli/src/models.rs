use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProfile {
    pub server: String,
    #[serde(alias = "token")]
    pub api_key: String,
    #[serde(default)]
    pub api_key_direct: bool,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: Option<i64>,
    pub current_repo: Option<String>,
    #[serde(default = "default_branch")]
    pub current_branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageFile {
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub assets: Vec<AssetDelta>,
}
impl StageFile {
    pub fn default_for_branch(branch: &str) -> Self {
        Self {
            branch: branch.to_string(),
            base_changeset_id: None,
            assets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDelta {
    pub path: String,
    pub blob_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceFile {
    pub path: String,
    pub blob_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub repo_id: String,
    pub branch: String,
    pub workspace_root: String,
    pub base_changeset_id: Option<String>,
    pub checked_out_assets: Vec<WorkspaceFile>,
    pub last_synced_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetStatusKind {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Staged,
    LockedByOther,
    StaleBase,
}
impl AssetStatusKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unmodified => "unmodified",
            Self::Modified => "modified",
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Staged => "staged",
            Self::LockedByOther => "locked_by_other",
            Self::StaleBase => "stale_base",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLockInfo {
    pub file_path: String,
    pub owner_id: String,
    pub locked_at: String,
    pub lease_expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeBlobRequest<'a> {
    pub manifest_hash: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeBlobResponse {
    pub blob_hash: String,
    pub size_bytes: u64,
}
