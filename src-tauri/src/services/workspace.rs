use std::{io, path::Path};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;

const SCHEMA_VERSION: i64 = 1;
const APP_SETTINGS_FILE: &str = "app_settings.json";

#[derive(Debug, Serialize, Deserialize)]
struct AppSettings {
    workspace_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceFile {
    workspace_path: String,
    schema_version: i64,
    updated_at: String,
}

pub async fn create_standard_dirs(root: &Path) -> io::Result<()> {
    if root.exists() && !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "workspace path points to a file",
        ));
    }

    fs::create_dir_all(root).await?;

    for segment in [
        "config",
        "db",
        "projects",
        "jobs",
        "artifacts",
        "logs",
        "temp",
        ".runtime",
    ] {
        fs::create_dir_all(root.join(segment)).await?;
    }

    Ok(())
}

pub async fn check_writable(root: &Path) -> bool {
    if !root.exists() || !root.is_dir() {
        return false;
    }

    let probe_path = root.join(".write_test");
    match fs::write(&probe_path, b"ok").await {
        Ok(()) => {
            let _ = fs::remove_file(probe_path).await;
            true
        }
        Err(_) => false,
    }
}

pub async fn write_workspace_json(root: &Path) -> io::Result<()> {
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).await?;

    let payload = WorkspaceFile {
        workspace_path: root.to_string_lossy().into_owned(),
        schema_version: SCHEMA_VERSION,
        updated_at: Utc::now().to_rfc3339(),
    };

    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    fs::write(config_dir.join("workspace.json"), bytes).await
}

pub async fn read_app_settings(data_dir: &Path) -> io::Result<Option<String>> {
    let settings_path = data_dir.join(APP_SETTINGS_FILE);
    if !fs::try_exists(&settings_path).await? {
        return Ok(None);
    }

    let bytes = fs::read(settings_path).await?;
    let settings: AppSettings = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if settings.workspace_path.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(settings.workspace_path))
}

pub async fn save_app_settings(data_dir: &Path, workspace_path: &str) -> io::Result<()> {
    fs::create_dir_all(data_dir).await?;

    let payload = AppSettings {
        workspace_path: workspace_path.to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    fs::write(data_dir.join(APP_SETTINGS_FILE), bytes).await
}