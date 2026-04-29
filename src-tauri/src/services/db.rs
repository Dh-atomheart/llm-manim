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