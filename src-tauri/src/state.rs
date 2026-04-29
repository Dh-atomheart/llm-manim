use sqlx::SqlitePool;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub workspace_path: Mutex<Option<String>>,
}

impl AppState {
    pub async fn get_db(&self) -> Option<SqlitePool> {
        self.db.lock().await.clone()
    }

    pub async fn get_workspace_path(&self) -> Option<String> {
        self.workspace_path.lock().await.clone()
    }

    pub async fn set_workspace(&self, workspace_path: String, pool: SqlitePool) {
        *self.workspace_path.lock().await = Some(workspace_path);
        *self.db.lock().await = Some(pool);
    }
}