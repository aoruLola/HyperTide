CREATE TABLE IF NOT EXISTS trust_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    log_head_hash TEXT NOT NULL,
    log_size BIGINT NOT NULL,
    state_root TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trust_checkpoints_created_at ON trust_checkpoints(created_at DESC);
