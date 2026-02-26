CREATE TABLE IF NOT EXISTS manifests (
    manifest_hash TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    chunk_size_policy TEXT NOT NULL,
    chunk_count INTEGER NOT NULL,
    manifest_json JSONB NOT NULL,
    merkle_root TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_manifests_created_at ON manifests(created_at DESC);
