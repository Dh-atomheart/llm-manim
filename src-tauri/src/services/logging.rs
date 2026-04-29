use std::{io, path::Path};

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use uuid::Uuid;

pub async fn write_provider_log(
    pool: Option<&SqlitePool>,
    workspace_root: Option<&Path>,
    level: &str,
    message: &str,
) {
    if let Some(pool) = pool {
        if let Err(error) = insert_job_log(pool, "provider", level, message).await {
            eprintln!("failed to insert provider log: {error}");
        }
    }

    if let Some(workspace_root) = workspace_root {
        if let Err(error) = append_app_log(workspace_root, "provider", level, message).await {
            eprintln!("failed to append provider app log: {error}");
        }
    }
}

async fn insert_job_log(
    pool: &SqlitePool,
    stage: &str,
    level: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let timestamp = Utc::now().to_rfc3339();
    let log_id = format!("log_{}", Uuid::new_v4());

    sqlx::query(
        "INSERT INTO job_logs (id, job_id, stage, level, message, timestamp) VALUES (?, NULL, ?, ?, ?, ?)",
    )
    .bind(log_id)
    .bind(stage)
    .bind(level)
    .bind(message)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(())
}

async fn append_app_log(
    workspace_root: &Path,
    stage: &str,
    level: &str,
    message: &str,
) -> io::Result<()> {
    let logs_dir = workspace_root.join("logs");
    fs::create_dir_all(&logs_dir).await?;

    let line = format!(
        "{}\t{}\t{}\t{}\n",
        Utc::now().to_rfc3339(),
        stage,
        level,
        message.replace('\n', " ")
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs_dir.join("app.log"))
        .await?;

    file.write_all(line.as_bytes()).await
}