DROP INDEX IF EXISTS idx_event_store_session;
DROP INDEX IF EXISTS idx_event_store_actor;

ALTER TABLE event_store
    DROP COLUMN IF EXISTS workflow_id,
    DROP COLUMN IF EXISTS tool_id,
    DROP COLUMN IF EXISTS session_id;
