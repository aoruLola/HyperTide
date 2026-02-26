DROP INDEX IF EXISTS idx_snapshots_repo_branch_asset;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'snapshots_repo_branch_changeset_asset_key'
    ) THEN
        ALTER TABLE snapshots
            DROP CONSTRAINT snapshots_repo_branch_changeset_asset_key;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'snapshots_repo_id_branch_name_changeset_id_path_key'
    ) THEN
        ALTER TABLE snapshots
            ADD CONSTRAINT snapshots_repo_id_branch_name_changeset_id_path_key
            UNIQUE (repo_id, branch_name, changeset_id, path);
    END IF;
END $$;

ALTER TABLE snapshots
    DROP COLUMN IF EXISTS asset_id;

DROP INDEX IF EXISTS idx_asset_deltas_asset_id;

ALTER TABLE asset_deltas
    DROP COLUMN IF EXISTS to_blob_hash,
    DROP COLUMN IF EXISTS from_blob_hash,
    DROP COLUMN IF EXISTS asset_id;
