DROP INDEX IF EXISTS idx_checkpoint_assets_checkpoint;
DROP INDEX IF EXISTS idx_session_checkpoints_repo_branch_created;
DROP INDEX IF EXISTS idx_session_checkpoints_session_created;
DROP INDEX IF EXISTS idx_agent_sessions_agent_run;
DROP INDEX IF EXISTS idx_agent_sessions_repo_branch_created;

DROP TABLE IF EXISTS checkpoint_assets;
DROP TABLE IF EXISTS session_checkpoints;
DROP TABLE IF EXISTS agent_sessions;
