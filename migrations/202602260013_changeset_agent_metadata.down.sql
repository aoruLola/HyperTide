DROP INDEX IF EXISTS idx_changesets_parent_checkpoint_id;
DROP INDEX IF EXISTS idx_changesets_session_id;
DROP INDEX IF EXISTS idx_changesets_agent_run_id;
DROP INDEX IF EXISTS idx_changesets_task_id;
DROP INDEX IF EXISTS idx_changesets_intent_id;

ALTER TABLE changesets
    DROP COLUMN IF EXISTS semantic_summary,
    DROP COLUMN IF EXISTS risk_level,
    DROP COLUMN IF EXISTS parent_checkpoint_id,
    DROP COLUMN IF EXISTS session_id,
    DROP COLUMN IF EXISTS agent_run_id,
    DROP COLUMN IF EXISTS task_id,
    DROP COLUMN IF EXISTS intent_id;
