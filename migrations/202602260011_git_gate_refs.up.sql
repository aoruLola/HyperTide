ALTER TABLE changesets
    ADD COLUMN IF NOT EXISTS staging_ref TEXT NULL,
    ADD COLUMN IF NOT EXISTS visible_ref TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_changesets_staging_ref ON changesets(staging_ref);
CREATE INDEX IF NOT EXISTS idx_changesets_visible_ref ON changesets(visible_ref);
