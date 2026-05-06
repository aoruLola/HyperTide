ALTER TABLE event_store
    ADD COLUMN IF NOT EXISTS workflow_id TEXT,
    ADD COLUMN IF NOT EXISTS tool_id TEXT,
    ADD COLUMN IF NOT EXISTS session_id TEXT;

CREATE INDEX IF NOT EXISTS idx_event_store_session
    ON event_store(session_id) WHERE session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_event_store_actor
    ON event_store(actor_id) WHERE actor_id IS NOT NULL;
