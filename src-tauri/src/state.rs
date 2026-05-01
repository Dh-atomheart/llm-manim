use std::sync::{atomic::AtomicBool, Arc};

use sqlx::SqlitePool;
use tokio::sync::{mpsc::UnboundedSender, Mutex};

#[derive(Default)]
pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub job_queue_tx: Mutex<Option<UnboundedSender<String>>>,
    pub running_job: Mutex<Option<(String, Arc<AtomicBool>)>>,
    pub workspace_path: Mutex<Option<String>>,
}

impl AppState {
    pub async fn get_db(&self) -> Option<SqlitePool> {
        self.db.lock().await.clone()
    }

    pub async fn get_workspace_path(&self) -> Option<String> {
        self.workspace_path.lock().await.clone()
    }

    pub async fn set_queue_sender(&self, queue_tx: UnboundedSender<String>) {
        *self.job_queue_tx.lock().await = Some(queue_tx);
    }

    pub async fn get_queue_sender(&self) -> Option<UnboundedSender<String>> {
        self.job_queue_tx.lock().await.clone()
    }

    pub async fn set_running_job(&self, job_id: String, cancel_flag: Arc<AtomicBool>) {
        *self.running_job.lock().await = Some((job_id, cancel_flag));
    }

    pub async fn get_running_job(&self) -> Option<(String, Arc<AtomicBool>)> {
        self.running_job.lock().await.clone()
    }

    pub async fn clear_running_job(&self) {
        *self.running_job.lock().await = None;
    }

    pub async fn set_workspace(&self, workspace_path: String, pool: SqlitePool) {
        *self.workspace_path.lock().await = Some(workspace_path);
        *self.db.lock().await = Some(pool);
    }
}
