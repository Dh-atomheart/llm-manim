CREATE TABLE IF NOT EXISTS render_artifacts (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL UNIQUE,
  project_id TEXT NOT NULL,
  file_path TEXT NOT NULL,
  duration_secs REAL NOT NULL,
  file_size_bytes INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (job_id) REFERENCES prompt_jobs (id),
  FOREIGN KEY (project_id) REFERENCES projects (id)
);

CREATE INDEX IF NOT EXISTS idx_render_artifacts_project_created_at
  ON render_artifacts (project_id, created_at DESC);
