ALTER TABLE prompt_jobs ADD COLUMN deleted_at TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_prompt_jobs_deleted_at
  ON prompt_jobs (deleted_at);
