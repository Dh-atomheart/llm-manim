ALTER TABLE provider_configs ADD COLUMN deleted_at TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_provider_configs_deleted_at
  ON provider_configs (deleted_at);

CREATE INDEX IF NOT EXISTS idx_prompt_jobs_provider_id
  ON prompt_jobs (provider_id);