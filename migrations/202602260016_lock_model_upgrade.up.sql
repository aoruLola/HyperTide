ALTER TABLE locks
    ADD COLUMN IF NOT EXISTS repo_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS scope TEXT NOT NULL DEFAULT 'asset';

-- Drop old unique constraint if it exists, add new composite one
ALTER TABLE locks DROP CONSTRAINT IF EXISTS locks_file_path_key;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'locks_repo_scope_path'
    ) THEN
        ALTER TABLE locks ADD CONSTRAINT locks_repo_scope_path
            UNIQUE (repo_id, scope, file_path);
    END IF;
END $$;
