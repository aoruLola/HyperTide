DROP INDEX IF EXISTS idx_changesets_status;

ALTER TABLE changesets
    DROP COLUMN IF EXISTS promoted_at,
    DROP COLUMN IF EXISTS approved_at,
    DROP COLUMN IF EXISTS approved_by,
    DROP COLUMN IF EXISTS status;
