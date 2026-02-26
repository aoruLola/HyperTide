CREATE TABLE IF NOT EXISTS audit_chain_entries (
    seq BIGSERIAL PRIMARY KEY,
    prev_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL UNIQUE,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    repo_id TEXT NULL,
    target_id TEXT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_chain_created_at ON audit_chain_entries(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_chain_action_created ON audit_chain_entries(action, created_at DESC);
