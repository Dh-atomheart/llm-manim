CREATE TABLE IF NOT EXISTS workspace_config (
  id TEXT PRIMARY KEY,
  workspace_path TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects (deleted_at);