CREATE TABLE IF NOT EXISTS event_store (
    event_id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    repo_id TEXT NULL,
    changeset_id TEXT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_store_created_at ON event_store(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_event_store_type_created ON event_store(event_type, created_at DESC);
