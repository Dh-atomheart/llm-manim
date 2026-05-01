use std::path::Path;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

pub async fn open_or_create(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    open_with_mode(path, true).await
}

pub async fn open_existing(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    open_with_mode(path, false).await
}

async fn open_with_mode(path: &Path, create_if_missing: bool) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(create_if_missing)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, env, path::PathBuf};

    use sqlx::Row;
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn migration_on_fresh_db_creates_required_tables() {
        let workspace_root = setup_workspace_root().await;
        let database_path = workspace_root.join("db").join("app.sqlite");

        let pool = open_or_create(&database_path).await.unwrap();
        let table_names: HashSet<String> =
            sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'")
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get("name"))
                .collect();

        for table_name in [
            "workspace_config",
            "projects",
            "provider_configs",
            "prompt_jobs",
            "render_artifacts",
            "job_logs",
            "generation_settings",
        ] {
            assert!(
                table_names.contains(table_name),
                "missing table: {table_name}"
            );
        }

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn migration_is_idempotent_for_existing_db() {
        let workspace_root = setup_workspace_root().await;
        let database_path = workspace_root.join("db").join("app.sqlite");

        let first_pool = open_or_create(&database_path).await.unwrap();
        sqlx::query("SELECT 1").execute(&first_pool).await.unwrap();
        drop(first_pool);

        let second_pool = open_or_create(&database_path).await.unwrap();
        let row = sqlx::query("SELECT 1 AS value")
            .fetch_one(&second_pool)
            .await
            .unwrap();

        assert_eq!(row.get::<i64, _>("value"), 1);

        cleanup(workspace_root).await;
    }

    async fn setup_workspace_root() -> PathBuf {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-db-tests-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(workspace_root.join("db"))
            .await
            .unwrap();
        workspace_root
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
