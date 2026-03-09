CREATE TABLE IF NOT EXISTS witness_receipts (
    id BIGSERIAL PRIMARY KEY,
    checkpoint_id TEXT NOT NULL REFERENCES trust_checkpoints(checkpoint_id) ON DELETE CASCADE,
    witness_id TEXT NOT NULL,
    signature TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (checkpoint_id, witness_id)
);

CREATE INDEX IF NOT EXISTS idx_witness_receipts_checkpoint ON witness_receipts(checkpoint_id);
