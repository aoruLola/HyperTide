use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::core::versioning::{
    AssetDelta, BranchRecord, BranchState, ChangesetKind, ChangesetRecord, RepoState,
    SnapshotAsset,
};

#[derive(Clone)]
pub struct VersionRepoPg {
    pool: PgPool,
}

#[derive(Debug, FromRow)]
struct RepoRow {
    repo_id: String,
    created_by: String,
}

#[derive(Debug, FromRow)]
struct BranchRow {
    branch_name: String,
    head_changeset_id: Option<String>,
    created_by: String,
    created_at: DateTime<Utc>,
    is_default: bool,
}

#[derive(Debug, FromRow)]
struct ChangesetRow {
    changeset_id: String,
    repo_id: String,
    branch_name: String,
    parent_changeset_id: Option<String>,
    base_changeset_id: Option<String>,
    kind: String,
    rollback_of: Option<String>,
    author: String,
    message: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct AssetDeltaRow {
    changeset_id: String,
    asset_id: Option<String>,
    path: String,
    from_blob_hash: Option<String>,
    to_blob_hash: Option<String>,
    blob_hash: Option<String>,
}

#[derive(Debug, FromRow)]
struct SnapshotRow {
    changeset_id: String,
    asset_id: Option<String>,
    path: String,
    blob_hash: String,
}

impl VersionRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn load_repos(&self) -> Result<HashMap<String, RepoState>, sqlx::Error> {
        let mut repos = HashMap::new();

        let repo_rows = sqlx::query_as::<_, RepoRow>(
            r#"
            SELECT repo_id, created_by
            FROM repos
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        for repo_row in repo_rows {
            let mut repo = RepoState {
                default_branch: "main".to_string(),
                branches: HashMap::new(),
                changesets: HashMap::new(),
                snapshots: HashMap::new(),
            };

            let branch_rows = sqlx::query_as::<_, BranchRow>(
                r#"
                SELECT branch_name, head_changeset_id, created_by, created_at, is_default
                FROM branches
                WHERE repo_id = $1
                ORDER BY created_at ASC
                "#,
            )
            .bind(&repo_row.repo_id)
            .fetch_all(&self.pool)
            .await?;

            for branch_row in branch_rows {
                let branch_name = branch_row.branch_name.clone();
                if branch_row.is_default {
                    repo.default_branch = branch_name.clone();
                }

                repo.branches.insert(
                    branch_name.clone(),
                    BranchState {
                        record: BranchRecord {
                            name: branch_name,
                            created_by: branch_row.created_by,
                            created_at: branch_row.created_at,
                            is_default: branch_row.is_default,
                            head_changeset_id: branch_row.head_changeset_id,
                        },
                        history: Vec::new(),
                    },
                );
            }

            if repo.branches.is_empty() {
                repo.ensure_default_branch(&repo_row.created_by);
            }

            let changeset_rows = sqlx::query_as::<_, ChangesetRow>(
                r#"
                SELECT changeset_id, repo_id, branch_name, parent_changeset_id, base_changeset_id, kind, rollback_of, author, message, created_at
                FROM changesets
                WHERE repo_id = $1
                ORDER BY created_at ASC
                "#,
            )
            .bind(&repo_row.repo_id)
            .fetch_all(&self.pool)
            .await?;

            for row in changeset_rows {
                repo.changesets.insert(
                    row.changeset_id.clone(),
                    ChangesetRecord {
                        changeset_id: row.changeset_id,
                        repo_id: row.repo_id,
                        branch: row.branch_name,
                        parent_changeset_id: row.parent_changeset_id,
                        base_changeset_id: row.base_changeset_id,
                        kind: parse_kind(&row.kind),
                        rollback_of: row.rollback_of,
                        author: row.author,
                        message: row.message,
                        created_at: row.created_at,
                        assets: Vec::new(),
                    },
                );
            }

            let delta_rows = sqlx::query_as::<_, AssetDeltaRow>(
                r#"
                SELECT d.changeset_id, d.asset_id, d.path, d.from_blob_hash, d.to_blob_hash, d.blob_hash
                FROM asset_deltas d
                INNER JOIN changesets c ON c.changeset_id = d.changeset_id
                WHERE c.repo_id = $1
                ORDER BY d.id ASC
                "#,
            )
            .bind(&repo_row.repo_id)
            .fetch_all(&self.pool)
            .await?;

            for delta in delta_rows {
                if let Some(changeset) = repo.changesets.get_mut(&delta.changeset_id) {
                    changeset.assets.push(AssetDelta {
                        asset_id: delta.asset_id,
                        path: delta.path,
                        from_blob_hash: delta.from_blob_hash,
                        blob_hash: delta.to_blob_hash.or(delta.blob_hash),
                    });
                }
            }

            let snapshot_rows = sqlx::query_as::<_, SnapshotRow>(
                r#"
                SELECT changeset_id, asset_id, path, blob_hash
                FROM snapshots
                WHERE repo_id = $1
                ORDER BY id ASC
                "#,
            )
            .bind(&repo_row.repo_id)
            .fetch_all(&self.pool)
            .await?;

            for row in snapshot_rows {
                let asset_id = row.asset_id.unwrap_or_else(|| row.path.clone());
                repo.snapshots
                    .entry(row.changeset_id)
                    .or_insert_with(HashMap::new)
                    .insert(
                        asset_id.clone(),
                        SnapshotAsset {
                            asset_id,
                            path: row.path,
                            blob_hash: row.blob_hash,
                        },
                    );
            }

            let branch_heads = repo
                .branches
                .iter()
                .map(|(name, state)| (name.clone(), state.record.head_changeset_id.clone()))
                .collect::<Vec<_>>();
            for (name, head) in branch_heads {
                let history = head
                    .as_deref()
                    .and_then(|head_id| repo.lineage_to(head_id))
                    .unwrap_or_default();
                if let Some(state) = repo.branches.get_mut(&name) {
                    state.history = history;
                }
            }

            repos.insert(repo_row.repo_id, repo);
        }

        Ok(repos)
    }

    pub(super) async fn replace_repo_state(
        &self,
        repo_id: &str,
        repo: &RepoState,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let created_by = repo
            .branches
            .get(&repo.default_branch)
            .map(|state| state.record.created_by.as_str())
            .or_else(|| repo.branches.values().next().map(|s| s.record.created_by.as_str()))
            .unwrap_or("system");

        sqlx::query(
            r#"
            INSERT INTO repos (repo_id, created_by)
            VALUES ($1, $2)
            ON CONFLICT (repo_id) DO UPDATE SET created_by = EXCLUDED.created_by
            "#,
        )
        .bind(repo_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM branches WHERE repo_id = $1")
            .bind(repo_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM changesets WHERE repo_id = $1")
            .bind(repo_id)
            .execute(&mut *tx)
            .await?;

        for changeset in repo.changesets.values() {
            sqlx::query(
                r#"
                INSERT INTO changesets (changeset_id, repo_id, branch_name, parent_changeset_id, base_changeset_id, kind, rollback_of, author, message, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(&changeset.changeset_id)
            .bind(&changeset.repo_id)
            .bind(&changeset.branch)
            .bind(&changeset.parent_changeset_id)
            .bind(&changeset.base_changeset_id)
            .bind(changeset.kind.as_str())
            .bind(&changeset.rollback_of)
            .bind(&changeset.author)
            .bind(&changeset.message)
            .bind(changeset.created_at)
            .execute(&mut *tx)
            .await?;

            for delta in &changeset.assets {
                sqlx::query(
                    r#"
                    INSERT INTO asset_deltas (changeset_id, asset_id, path, from_blob_hash, to_blob_hash, blob_hash)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                )
                .bind(&changeset.changeset_id)
                .bind(delta.asset_id.as_deref().unwrap_or(&delta.path))
                .bind(&delta.path)
                .bind(&delta.from_blob_hash)
                .bind(&delta.blob_hash)
                .bind(&delta.blob_hash)
                .execute(&mut *tx)
                .await?;
            }
        }

        for (changeset_id, snapshot) in &repo.snapshots {
            for (asset_id, snapshot_asset) in snapshot {
                sqlx::query(
                    r#"
                    INSERT INTO snapshots (repo_id, branch_name, changeset_id, asset_id, path, blob_hash)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                )
                .bind(repo_id)
                .bind(&repo.default_branch)
                .bind(changeset_id)
                .bind(asset_id)
                .bind(&snapshot_asset.path)
                .bind(&snapshot_asset.blob_hash)
                .execute(&mut *tx)
                .await?;
            }
        }

        for branch in repo.branches.values() {
            sqlx::query(
                r#"
                INSERT INTO branches (repo_id, branch_name, head_changeset_id, created_by, created_at, is_default)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(repo_id)
            .bind(&branch.record.name)
            .bind(&branch.record.head_changeset_id)
            .bind(&branch.record.created_by)
            .bind(branch.record.created_at)
            .bind(branch.record.is_default)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

fn parse_kind(value: &str) -> ChangesetKind {
    match value {
        "rollback" => ChangesetKind::Rollback,
        _ => ChangesetKind::Normal,
    }
}
