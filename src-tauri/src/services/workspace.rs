use std::{
    io,
    path::Path,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;

const SCHEMA_VERSION: i64 = 1;
const APP_SETTINGS_FILE: &str = "app_settings.json";
const TEMP_RETENTION_SECS: u64 = 48 * 60 * 60;

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

pub async fn clean_temp_dir(root: &Path) -> io::Result<()> {
    clean_temp_dir_with_clock(
        root,
        SystemTime::now(),
        Duration::from_secs(TEMP_RETENTION_SECS),
    )
    .await
}

async fn clean_temp_dir_with_clock(
    root: &Path,
    now: SystemTime,
    retention: Duration,
) -> io::Result<()> {
    let temp_dir = root.join("temp");
    if !fs::try_exists(&temp_dir).await? {
        return Ok(());
    }

    let mut entries = fs::read_dir(&temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        let Ok(modified_at) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified_at) else {
            continue;
        };

        if age <= retention {
            continue;
        }

        if metadata.is_dir() {
            fs::remove_dir_all(path).await?;
        } else {
            fs::remove_file(path).await?;
        }
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

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn clean_temp_removes_stale_files() {
        let workspace_root = setup_workspace_root().await;
        let stale_file = workspace_root.join("temp").join("stale.txt");
        fs::write(&stale_file, b"stale").await.unwrap();

        let modified_at = fs::metadata(&stale_file).await.unwrap().modified().unwrap();
        clean_temp_dir_with_clock(
            &workspace_root,
            modified_at + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .await
        .unwrap();

        assert!(!fs::try_exists(&stale_file).await.unwrap());

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn clean_temp_preserves_recent_files() {
        let workspace_root = setup_workspace_root().await;
        let recent_file = workspace_root.join("temp").join("recent.txt");
        fs::write(&recent_file, b"recent").await.unwrap();

        let modified_at = fs::metadata(&recent_file)
            .await
            .unwrap()
            .modified()
            .unwrap();
        clean_temp_dir_with_clock(&workspace_root, modified_at, Duration::from_secs(60))
            .await
            .unwrap();

        assert!(fs::try_exists(&recent_file).await.unwrap());

        cleanup(workspace_root).await;
    }

    async fn setup_workspace_root() -> PathBuf {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-workspace-tests-{}", Uuid::new_v4()));
        create_standard_dirs(&workspace_root).await.unwrap();
        workspace_root
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = fs::remove_dir_all(workspace_root).await;
    }
}
