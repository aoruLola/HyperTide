use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ROOT_BASE_CHANGESET_ID: &str = "ROOT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangesetKind {
    Normal,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDelta {
    pub path: String,
    pub blob_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesetRecord {
    pub changeset_id: String,
    pub repo_id: String,
    pub branch: String,
    pub parent_changeset_id: Option<String>,
    pub base_changeset_id: Option<String>,
    pub kind: ChangesetKind,
    pub rollback_of: Option<String>,
    pub author: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRecord {
    pub name: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub is_default: bool,
    pub head_changeset_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubmitChangesetInput {
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: Option<String>,
    pub kind: ChangesetKind,
    pub rollback_of: Option<String>,
    pub author: String,
    pub message: String,
    pub assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryPage {
    pub items: Vec<ChangesetRecord>,
    pub next_cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncSnapshot {
    pub repo_id: String,
    pub branch: String,
    pub changeset_id: Option<String>,
    pub assets: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotEntry {
    pub path: String,
    pub blob_hash: String,
}

#[derive(Debug, Clone)]
pub struct RollbackPlan {
    pub repo_id: String,
    pub branch: String,
    pub base_changeset_id: String,
    pub target_changeset_id: String,
    pub assets: Vec<AssetDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersioningError {
    RepoNotFound {
        repo_id: String,
    },
    BranchNotFound {
        repo_id: String,
        branch: String,
    },
    BranchAlreadyExists {
        repo_id: String,
        branch: String,
    },
    ChangesetNotFound {
        repo_id: String,
        changeset_id: String,
    },
    BaseChangesetRequired,
    BaseChangesetMismatch {
        repo_id: String,
        branch: String,
        expected: Option<String>,
        got: Option<String>,
    },
    InvalidRollbackTarget {
        repo_id: String,
        branch: String,
        target_changeset_id: String,
    },
}

#[derive(Clone)]
pub struct VersionManager {
    repos: Arc<RwLock<HashMap<String, RepoState>>>,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            repos: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_branch(
        &self,
        repo_id: &str,
        branch: &str,
        from_changeset_id: Option<&str>,
        created_by: &str,
    ) -> Result<BranchRecord, VersioningError> {
        let mut repos = self.repos.write().expect("versioning lock poisoned");
        let repo = repos
            .entry(repo_id.to_string())
            .or_insert_with(|| RepoState::new(created_by));
        repo.ensure_default_branch(created_by);

        if repo.branches.contains_key(branch) {
            return Err(VersioningError::BranchAlreadyExists {
                repo_id: repo_id.to_string(),
                branch: branch.to_string(),
            });
        }

        let head = if let Some(id) = from_changeset_id {
            if !repo.changesets.contains_key(id) {
                return Err(VersioningError::ChangesetNotFound {
                    repo_id: repo_id.to_string(),
                    changeset_id: id.to_string(),
                });
            }
            Some(id.to_string())
        } else {
            repo.default_head()
        };

        let history = if let Some(ref head_id) = head {
            repo.lineage_to(head_id)
                .ok_or_else(|| VersioningError::ChangesetNotFound {
                    repo_id: repo_id.to_string(),
                    changeset_id: head_id.clone(),
                })?
        } else {
            Vec::new()
        };

        let record = BranchRecord {
            name: branch.to_string(),
            created_by: created_by.to_string(),
            created_at: Utc::now(),
            is_default: false,
            head_changeset_id: head.clone(),
        };

        repo.branches.insert(
            branch.to_string(),
            BranchState {
                record: record.clone(),
                history,
            },
        );

        Ok(record)
    }

    pub fn list_branches(&self, repo_id: &str) -> Result<Vec<BranchRecord>, VersioningError> {
        let repos = self.repos.read().expect("versioning lock poisoned");
        let repo = repos
            .get(repo_id)
            .ok_or_else(|| VersioningError::RepoNotFound {
                repo_id: repo_id.to_string(),
            })?;

        let mut items: Vec<BranchRecord> =
            repo.branches.values().map(|b| b.record.clone()).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(items)
    }

    pub fn submit_changeset(
        &self,
        input: SubmitChangesetInput,
    ) -> Result<ChangesetRecord, VersioningError> {
        let mut repos = self.repos.write().expect("versioning lock poisoned");
        let repo = repos
            .entry(input.repo_id.clone())
            .or_insert_with(|| RepoState::new(&input.author));
        repo.ensure_default_branch(&input.author);
        Self::submit_internal(repo, input)
    }

    pub fn history(
        &self,
        repo_id: &str,
        branch: &str,
        limit: usize,
        cursor: usize,
    ) -> Result<HistoryPage, VersioningError> {
        let repos = self.repos.read().expect("versioning lock poisoned");
        let repo = repos
            .get(repo_id)
            .ok_or_else(|| VersioningError::RepoNotFound {
                repo_id: repo_id.to_string(),
            })?;
        let branch_state =
            repo.branches
                .get(branch)
                .ok_or_else(|| VersioningError::BranchNotFound {
                    repo_id: repo_id.to_string(),
                    branch: branch.to_string(),
                })?;

        let total = branch_state.history.len();
        let max_limit = limit.clamp(1, 200);
        let items: Vec<ChangesetRecord> = branch_state
            .history
            .iter()
            .rev()
            .skip(cursor)
            .take(max_limit)
            .filter_map(|id| repo.changesets.get(id).cloned())
            .collect();

        let consumed = cursor + items.len();
        let next_cursor = if consumed < total {
            Some(consumed)
        } else {
            None
        };
        Ok(HistoryPage { items, next_cursor })
    }

    pub fn build_rollback_plan(
        &self,
        repo_id: &str,
        branch: &str,
        target_changeset_id: &str,
    ) -> Result<RollbackPlan, VersioningError> {
        let repos = self.repos.read().expect("versioning lock poisoned");
        let repo = repos
            .get(repo_id)
            .ok_or_else(|| VersioningError::RepoNotFound {
                repo_id: repo_id.to_string(),
            })?;
        let branch_state =
            repo.branches
                .get(branch)
                .ok_or_else(|| VersioningError::BranchNotFound {
                    repo_id: repo_id.to_string(),
                    branch: branch.to_string(),
                })?;

        let head_id = branch_state
            .record
            .head_changeset_id
            .clone()
            .ok_or_else(|| VersioningError::InvalidRollbackTarget {
                repo_id: repo_id.to_string(),
                branch: branch.to_string(),
                target_changeset_id: target_changeset_id.to_string(),
            })?;

        if head_id == target_changeset_id {
            return Err(VersioningError::InvalidRollbackTarget {
                repo_id: repo_id.to_string(),
                branch: branch.to_string(),
                target_changeset_id: target_changeset_id.to_string(),
            });
        }

        if !branch_state
            .history
            .iter()
            .any(|id| id == target_changeset_id)
        {
            return Err(VersioningError::InvalidRollbackTarget {
                repo_id: repo_id.to_string(),
                branch: branch.to_string(),
                target_changeset_id: target_changeset_id.to_string(),
            });
        }

        let current = repo.snapshots.get(&head_id).cloned().unwrap_or_default();
        let target = repo
            .snapshots
            .get(target_changeset_id)
            .cloned()
            .ok_or_else(|| VersioningError::ChangesetNotFound {
                repo_id: repo_id.to_string(),
                changeset_id: target_changeset_id.to_string(),
            })?;

        let mut paths = BTreeSet::new();
        current.keys().for_each(|k| {
            paths.insert(k.clone());
        });
        target.keys().for_each(|k| {
            paths.insert(k.clone());
        });

        let mut assets = Vec::new();
        for path in paths {
            let current_hash = current.get(&path);
            let target_hash = target.get(&path);
            if current_hash == target_hash {
                continue;
            }
            assets.push(AssetDelta {
                path,
                blob_hash: target_hash.cloned(),
            });
        }

        Ok(RollbackPlan {
            repo_id: repo_id.to_string(),
            branch: branch.to_string(),
            base_changeset_id: head_id,
            target_changeset_id: target_changeset_id.to_string(),
            assets,
        })
    }

    pub fn sync_snapshot(
        &self,
        repo_id: &str,
        branch: &str,
        to_changeset_id: Option<&str>,
    ) -> Result<SyncSnapshot, VersioningError> {
        let repos = self.repos.read().expect("versioning lock poisoned");
        let repo = repos
            .get(repo_id)
            .ok_or_else(|| VersioningError::RepoNotFound {
                repo_id: repo_id.to_string(),
            })?;
        let branch_state =
            repo.branches
                .get(branch)
                .ok_or_else(|| VersioningError::BranchNotFound {
                    repo_id: repo_id.to_string(),
                    branch: branch.to_string(),
                })?;

        let chosen = if let Some(id) = to_changeset_id {
            if !branch_state.history.iter().any(|entry| entry == id) {
                return Err(VersioningError::ChangesetNotFound {
                    repo_id: repo_id.to_string(),
                    changeset_id: id.to_string(),
                });
            }
            Some(id.to_string())
        } else {
            branch_state.record.head_changeset_id.clone()
        };

        let snapshot_map = chosen
            .as_ref()
            .and_then(|id| repo.snapshots.get(id))
            .cloned()
            .unwrap_or_default();
        let mut assets: Vec<SnapshotEntry> = snapshot_map
            .into_iter()
            .map(|(path, blob_hash)| SnapshotEntry { path, blob_hash })
            .collect();
        assets.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(SyncSnapshot {
            repo_id: repo_id.to_string(),
            branch: branch.to_string(),
            changeset_id: chosen,
            assets,
        })
    }

    fn submit_internal(
        repo: &mut RepoState,
        input: SubmitChangesetInput,
    ) -> Result<ChangesetRecord, VersioningError> {
        if input.base_changeset_id.is_none() {
            return Err(VersioningError::BaseChangesetRequired);
        }

        let branch_state = repo.branches.get_mut(&input.branch).ok_or_else(|| {
            VersioningError::BranchNotFound {
                repo_id: input.repo_id.clone(),
                branch: input.branch.clone(),
            }
        })?;

        let expected = branch_state.record.head_changeset_id.clone();
        if expected.is_none() {
            if input.base_changeset_id.as_deref() != Some(ROOT_BASE_CHANGESET_ID) {
                return Err(VersioningError::BaseChangesetMismatch {
                    repo_id: input.repo_id,
                    branch: input.branch,
                    expected,
                    got: input.base_changeset_id,
                });
            }
        } else if input.base_changeset_id != expected {
            return Err(VersioningError::BaseChangesetMismatch {
                repo_id: input.repo_id,
                branch: input.branch,
                expected,
                got: input.base_changeset_id,
            });
        }

        let parent_changeset_id = branch_state.record.head_changeset_id.clone();
        let mut new_snapshot = parent_changeset_id
            .as_ref()
            .and_then(|id| repo.snapshots.get(id))
            .cloned()
            .unwrap_or_default();

        for asset in &input.assets {
            if let Some(hash) = &asset.blob_hash {
                new_snapshot.insert(asset.path.clone(), hash.clone());
            } else {
                new_snapshot.remove(&asset.path);
            }
        }

        let changeset_id = Uuid::new_v4().to_string();
        let record = ChangesetRecord {
            changeset_id: changeset_id.clone(),
            repo_id: input.repo_id,
            branch: input.branch.clone(),
            parent_changeset_id,
            base_changeset_id: input.base_changeset_id,
            kind: input.kind,
            rollback_of: input.rollback_of,
            author: input.author,
            message: input.message,
            created_at: Utc::now(),
            assets: input.assets,
        };

        repo.snapshots.insert(changeset_id.clone(), new_snapshot);
        repo.changesets.insert(changeset_id.clone(), record.clone());
        branch_state.record.head_changeset_id = Some(changeset_id.clone());
        branch_state.history.push(changeset_id);

        Ok(record)
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct BranchState {
    record: BranchRecord,
    history: Vec<String>,
}

#[derive(Debug, Clone)]
struct RepoState {
    default_branch: String,
    branches: HashMap<String, BranchState>,
    changesets: HashMap<String, ChangesetRecord>,
    snapshots: HashMap<String, HashMap<String, String>>,
}

impl RepoState {
    fn new(created_by: &str) -> Self {
        let mut repo = Self {
            default_branch: "main".to_string(),
            branches: HashMap::new(),
            changesets: HashMap::new(),
            snapshots: HashMap::new(),
        };
        repo.ensure_default_branch(created_by);
        repo
    }

    fn ensure_default_branch(&mut self, created_by: &str) {
        if self.branches.contains_key(&self.default_branch) {
            return;
        }
        let record = BranchRecord {
            name: self.default_branch.clone(),
            created_by: created_by.to_string(),
            created_at: Utc::now(),
            is_default: true,
            head_changeset_id: None,
        };
        self.branches.insert(
            self.default_branch.clone(),
            BranchState {
                record,
                history: Vec::new(),
            },
        );
    }

    fn default_head(&self) -> Option<String> {
        self.branches
            .get(&self.default_branch)
            .and_then(|branch| branch.record.head_changeset_id.clone())
    }

    fn lineage_to(&self, changeset_id: &str) -> Option<Vec<String>> {
        let mut chain = Vec::new();
        let mut current = Some(changeset_id.to_string());
        while let Some(id) = current {
            let node = self.changesets.get(&id)?;
            chain.push(id);
            current = node.parent_changeset_id.clone();
        }
        chain.reverse();
        Some(chain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_with_head_match_advances_branch_head() {
        let manager = VersionManager::new();

        let c1 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-a".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(ROOT_BASE_CHANGESET_ID.to_string()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "first".to_string(),
                assets: vec![AssetDelta {
                    path: "a.txt".to_string(),
                    blob_hash: Some("hash-1".to_string()),
                }],
            })
            .expect("first commit should succeed");

        let c2 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-a".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(c1.changeset_id.clone()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "second".to_string(),
                assets: vec![AssetDelta {
                    path: "a.txt".to_string(),
                    blob_hash: Some("hash-2".to_string()),
                }],
            })
            .expect("second commit should succeed");

        let sync = manager
            .sync_snapshot("repo-a", "main", None)
            .expect("snapshot should exist");
        assert_eq!(sync.changeset_id, Some(c2.changeset_id));
        assert_eq!(sync.assets.len(), 1);
        assert_eq!(sync.assets[0].blob_hash, "hash-2");
    }

    #[test]
    fn stale_base_is_rejected() {
        let manager = VersionManager::new();

        let c1 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-b".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(ROOT_BASE_CHANGESET_ID.to_string()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "first".to_string(),
                assets: vec![],
            })
            .expect("first should succeed");

        let c2 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-b".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(ROOT_BASE_CHANGESET_ID.to_string()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "invalid".to_string(),
                assets: vec![],
            })
            .expect_err("stale base must fail");

        assert_eq!(
            c2,
            VersioningError::BaseChangesetMismatch {
                repo_id: "repo-b".to_string(),
                branch: "main".to_string(),
                expected: Some(c1.changeset_id),
                got: Some(ROOT_BASE_CHANGESET_ID.to_string()),
            }
        );
    }

    #[test]
    fn rollback_plan_targets_existing_history() {
        let manager = VersionManager::new();
        let c1 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-c".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(ROOT_BASE_CHANGESET_ID.to_string()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "first".to_string(),
                assets: vec![AssetDelta {
                    path: "a".to_string(),
                    blob_hash: Some("h1".to_string()),
                }],
            })
            .expect("first commit");

        let c2 = manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-c".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(c1.changeset_id.clone()),
                kind: ChangesetKind::Normal,
                rollback_of: None,
                author: "alice".to_string(),
                message: "second".to_string(),
                assets: vec![AssetDelta {
                    path: "a".to_string(),
                    blob_hash: Some("h2".to_string()),
                }],
            })
            .expect("second commit");

        let plan = manager
            .build_rollback_plan("repo-c", "main", &c1.changeset_id)
            .expect("rollback plan");
        assert_eq!(plan.base_changeset_id, c2.changeset_id.clone());
        assert_eq!(plan.assets.len(), 1);
        assert_eq!(plan.assets[0].blob_hash.as_deref(), Some("h1"));

        manager
            .submit_changeset(SubmitChangesetInput {
                repo_id: "repo-c".to_string(),
                branch: "main".to_string(),
                base_changeset_id: Some(plan.base_changeset_id.clone()),
                kind: ChangesetKind::Rollback,
                rollback_of: Some(plan.target_changeset_id),
                author: "alice".to_string(),
                message: "rollback".to_string(),
                assets: plan.assets,
            })
            .expect("rollback commit should be accepted");

        let sync = manager
            .sync_snapshot("repo-c", "main", None)
            .expect("snapshot");
        assert_eq!(sync.assets[0].blob_hash, "h1");
    }
}
