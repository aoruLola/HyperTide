CREATE TABLE IF NOT EXISTS agent_sessions (
    session_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    base_changeset_id TEXT NULL,
    workspace_root TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    intent_id TEXT NULL,
    task_id TEXT NULL,
    agent_run_id TEXT NULL,
    trigger_reason TEXT NULL,
    risk_level TEXT NULL,
    semantic_summary TEXT NULL,
    expires_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS session_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES agent_sessions(session_id) ON DELETE CASCADE,
    repo_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    base_changeset_id TEXT NULL,
    parent_checkpoint_id TEXT NULL REFERENCES session_checkpoints(checkpoint_id),
    workspace_root TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    trigger_reason TEXT NOT NULL,
    intent_id TEXT NULL,
    task_id TEXT NULL,
    agent_run_id TEXT NULL,
    risk_level TEXT NULL,
    semantic_summary TEXT NULL,
    expires_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS checkpoint_assets (
    id BIGSERIAL PRIMARY KEY,
    checkpoint_id TEXT NOT NULL REFERENCES session_checkpoints(checkpoint_id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
    path TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    UNIQUE (checkpoint_id, asset_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_repo_branch_created
    ON agent_sessions(repo_id, branch_name, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_agent_run
    ON agent_sessions(agent_run_id);

CREATE INDEX IF NOT EXISTS idx_session_checkpoints_session_created
    ON session_checkpoints(session_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_session_checkpoints_repo_branch_created
    ON session_checkpoints(repo_id, branch_name, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_checkpoint_assets_checkpoint
    ON checkpoint_assets(checkpoint_id);
