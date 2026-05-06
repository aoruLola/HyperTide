ALTER TABLE asset_deltas
    ADD COLUMN IF NOT EXISTS asset_id TEXT,
    ADD COLUMN IF NOT EXISTS from_blob_hash TEXT,
    ADD COLUMN IF NOT EXISTS to_blob_hash TEXT;

UPDATE asset_deltas
SET asset_id = path
WHERE asset_id IS NULL;

UPDATE asset_deltas
SET to_blob_hash = blob_hash
WHERE to_blob_hash IS NULL;

ALTER TABLE asset_deltas
    ALTER COLUMN asset_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_asset_deltas_asset_id ON asset_deltas(asset_id);

ALTER TABLE snapshots
    ADD COLUMN IF NOT EXISTS asset_id TEXT;

UPDATE snapshots
SET asset_id = path
WHERE asset_id IS NULL;

ALTER TABLE snapshots
    ALTER COLUMN asset_id SET NOT NULL;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'snapshots_repo_id_branch_name_changeset_id_path_key'
    ) THEN
        ALTER TABLE snapshots
            DROP CONSTRAINT snapshots_repo_id_branch_name_changeset_id_path_key;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'snapshots_repo_branch_changeset_asset_key'
    ) THEN
        ALTER TABLE snapshots
            ADD CONSTRAINT snapshots_repo_branch_changeset_asset_key
            UNIQUE (repo_id, branch_name, changeset_id, asset_id);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_snapshots_repo_branch_asset
    ON snapshots(repo_id, branch_name, asset_id);
