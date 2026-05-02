ALTER TABLE changesets
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'visible',
    ADD COLUMN IF NOT EXISTS approved_by TEXT NULL,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS promoted_at TIMESTAMPTZ NULL;

CREATE INDEX IF NOT EXISTS idx_changesets_status ON changesets(status);
