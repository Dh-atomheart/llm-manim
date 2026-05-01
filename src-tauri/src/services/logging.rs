use std::{io, path::Path};

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use uuid::Uuid;

use crate::services::redact;

pub async fn write_provider_log(
    pool: Option<&SqlitePool>,
    workspace_root: Option<&Path>,
    level: &str,
    message: &str,
) {
    let sanitized = redact::redact(message, &[]);

    if let Some(pool) = pool {
        if let Err(error) = insert_log(pool, None, "provider", level, &sanitized).await {
            eprintln!("failed to insert provider log: {error}");
        }
    }

    if let Some(workspace_root) = workspace_root {
        if let Err(error) = append_app_log(workspace_root, "provider", level, &sanitized).await {
            eprintln!("failed to append provider app log: {error}");
        }
    }
}

pub async fn write_job_log(
    pool: Option<&SqlitePool>,
    workspace_root: Option<&Path>,
    job_id: &str,
    stage: &str,
    level: &str,
    message: &str,
) {
    let sanitized = redact::redact(message, &[]);

    if let Some(pool) = pool {
        if let Err(error) = insert_log(pool, Some(job_id), stage, level, &sanitized).await {
            eprintln!("failed to insert job log: {error}");
        }
    }

    if let Some(workspace_root) = workspace_root {
        if let Err(error) = append_app_log(workspace_root, stage, level, &sanitized).await {
            eprintln!("failed to append job app log: {error}");
        }
    }
}

async fn insert_log(
    pool: &SqlitePool,
    job_id: Option<&str>,
    stage: &str,
    level: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let timestamp = Utc::now().to_rfc3339();
    let log_id = format!("log_{}", Uuid::new_v4());

    sqlx::query(
        "INSERT INTO job_logs (id, job_id, stage, level, message, timestamp) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(log_id)
    .bind(job_id)
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

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use chrono::Utc;
    use sqlx::{Row, SqlitePool};
    use uuid::Uuid;

    use super::*;
    use crate::services::{db, workspace};

    #[tokio::test]
    async fn write_job_log_redacts_secrets_in_database_and_file_log() {
        let (pool, workspace_root, job_id) = setup_logging_context().await;
        let secret_message = concat!(
            "Authorization: Bearer sk-secret-token; ",
            "url=https://example.com?api_key=sk-secret-token; ",
            "payload={\"api_key\":\"sk-secret-token\"}",
        );

        write_job_log(
            Some(&pool),
            Some(&workspace_root),
            &job_id,
            "provider",
            "error",
            secret_message,
        )
        .await;

        let stored_message: String = sqlx::query(
            "SELECT message FROM job_logs WHERE job_id = ? ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("message");
        let app_log = fs::read_to_string(workspace_root.join("logs").join("app.log"))
            .await
            .unwrap();

        for content in [&stored_message, &app_log] {
            assert!(!content.contains("sk-secret-token"));
            assert!(content.contains("[REDACTED]"));
        }

        cleanup(workspace_root).await;
    }

    async fn setup_logging_context() -> (SqlitePool, PathBuf, String) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-logging-tests-{}", Uuid::new_v4()));
        workspace::create_standard_dirs(&workspace_root)
            .await
            .unwrap();

        let pool = db::open_or_create(&workspace_root.join("db").join("app.sqlite"))
            .await
            .unwrap();

        let provider_id = format!("provider_{}", Uuid::new_v4());
        let project_id = format!("project_{}", Uuid::new_v4());
        let job_id = format!("job_{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO provider_configs (id, name, provider_type, base_url, model, api_key, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(&provider_id)
        .bind("Provider")
        .bind("openai_compatible")
        .bind("https://api.example.com")
        .bind("test-model")
        .bind("sk-test-provider")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO projects (id, name, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, NULL)",
        )
        .bind(&project_id)
        .bind("Test Project")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) VALUES (?, ?, ?, ?, 'running', NULL, NULL, NULL, NULL, ?, ?, NULL)",
        )
        .bind(&job_id)
        .bind(&project_id)
        .bind(&provider_id)
        .bind("Write a log")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        (pool, workspace_root, job_id)
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = fs::remove_dir_all(workspace_root).await;
    }
}
