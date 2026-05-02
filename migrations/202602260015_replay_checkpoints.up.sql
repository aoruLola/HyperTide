CREATE TABLE IF NOT EXISTS replay_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    event_seq BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_replay_checkpoints_seq ON replay_checkpoints(event_seq);
