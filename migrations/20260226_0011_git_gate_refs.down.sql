DROP INDEX IF EXISTS idx_changesets_visible_ref;
DROP INDEX IF EXISTS idx_changesets_staging_ref;

ALTER TABLE changesets
    DROP COLUMN IF EXISTS visible_ref,
    DROP COLUMN IF EXISTS staging_ref;
