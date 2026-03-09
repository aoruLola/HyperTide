CREATE TABLE IF NOT EXISTS principals (
    principal_id TEXT PRIMARY KEY,
    principal_type TEXT NOT NULL DEFAULT 'service',
    display_name TEXT NOT NULL,
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS api_keys (
    key_hash TEXT PRIMARY KEY,
    principal_id TEXT NOT NULL REFERENCES principals(principal_id),
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    expires_at TIMESTAMPTZ NULL,
    revoked_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    token_hash TEXT PRIMARY KEY,
    principal_id TEXT NOT NULL REFERENCES principals(principal_id),
    family_id TEXT NOT NULL,
    parent_token_hash TEXT NULL REFERENCES refresh_tokens(token_hash),
    replaced_by_token_hash TEXT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL
);

CREATE TABLE IF NOT EXISTS locks (
    file_path TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    locked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lease_expires_at TIMESTAMPTZ NULL,
    force_released BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS repos (
    repo_id TEXT PRIMARY KEY,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS branches (
    repo_id TEXT NOT NULL REFERENCES repos(repo_id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL,
    head_changeset_id TEXT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (repo_id, branch_name)
);

CREATE TABLE IF NOT EXISTS changesets (
    changeset_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL REFERENCES repos(repo_id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL,
    parent_changeset_id TEXT NULL,
    base_changeset_id TEXT NULL,
    kind TEXT NOT NULL,
    rollback_of TEXT NULL,
    author TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS asset_deltas (
    id BIGSERIAL PRIMARY KEY,
    changeset_id TEXT NOT NULL REFERENCES changesets(changeset_id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    blob_hash TEXT NULL
);

CREATE TABLE IF NOT EXISTS snapshots (
    id BIGSERIAL PRIMARY KEY,
    repo_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    changeset_id TEXT NOT NULL REFERENCES changesets(changeset_id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (repo_id, branch_name, changeset_id, path)
);

CREATE TABLE IF NOT EXISTS audit_logs (
    audit_id BIGSERIAL PRIMARY KEY,
    actor_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_principal ON api_keys(principal_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_principal ON refresh_tokens(principal_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family ON refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_changesets_repo_branch_created ON changesets(repo_id, branch_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_asset_deltas_changeset ON asset_deltas(changeset_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_repo_branch_changeset ON snapshots(repo_id, branch_name, changeset_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at DESC);
