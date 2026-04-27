ALTER TABLE changesets
    ADD COLUMN IF NOT EXISTS intent_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS task_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS agent_run_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS session_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS parent_checkpoint_id TEXT NULL,
    ADD COLUMN IF NOT EXISTS risk_level TEXT NULL,
    ADD COLUMN IF NOT EXISTS semantic_summary TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_changesets_intent_id ON changesets(intent_id);
CREATE INDEX IF NOT EXISTS idx_changesets_task_id ON changesets(task_id);
CREATE INDEX IF NOT EXISTS idx_changesets_agent_run_id ON changesets(agent_run_id);
CREATE INDEX IF NOT EXISTS idx_changesets_session_id ON changesets(session_id);
CREATE INDEX IF NOT EXISTS idx_changesets_parent_checkpoint_id ON changesets(parent_checkpoint_id);
