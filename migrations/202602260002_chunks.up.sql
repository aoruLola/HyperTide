CREATE TABLE IF NOT EXISTS chunks (
    chunk_hash TEXT PRIMARY KEY,
    size_bytes BIGINT NOT NULL,
    algo TEXT NOT NULL DEFAULT 'blake3-v1',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chunks_created_at ON chunks(created_at DESC);
