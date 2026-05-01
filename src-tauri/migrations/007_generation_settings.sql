CREATE TABLE IF NOT EXISTS generation_settings (
  id TEXT PRIMARY KEY,
  strict_api_name_validation INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
