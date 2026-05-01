CREATE TABLE IF NOT EXISTS prompt_jobs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  prompt_text TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
  error_code TEXT NULL,
  error_summary TEXT NULL,
  suggestion TEXT NULL,
  retry_of_job_id TEXT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT NULL,
  finished_at TEXT NULL,
  FOREIGN KEY (project_id) REFERENCES projects (id),
  FOREIGN KEY (provider_id) REFERENCES provider_configs (id),
  FOREIGN KEY (retry_of_job_id) REFERENCES prompt_jobs (id)
);

CREATE INDEX IF NOT EXISTS idx_prompt_jobs_project_created_at
  ON prompt_jobs (project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_prompt_jobs_state_created_at
  ON prompt_jobs (state, created_at DESC);
