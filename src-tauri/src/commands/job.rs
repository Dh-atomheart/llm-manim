use std::{
    path::{Path, PathBuf},
    process::Command,
};

use chrono::Utc;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use tauri::State;
use uuid::Uuid;

use crate::{
    services::{logging, queue},
    state::AppState,
    types::{
        error_codes::{
            E_ARTIFACT_INVALID, E_CANCELLED, E_CANCEL_FAILED, E_DB, E_IO, E_JOB_NOT_CANCELLABLE,
            E_JOB_NOT_DELETABLE, E_JOB_NOT_RETRYABLE, E_NOT_FOUND, E_VALIDATION,
            E_WORKSPACE_INVALID,
        },
        response::{AppError, AppResponse},
    },
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPromptJobResult {
    job_id: String,
    state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptJobView {
    id: String,
    project_id: String,
    provider_id: String,
    prompt_text: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_of_job_id: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelJobResult {
    job_id: String,
    state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteJobResult {
    deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryJobResult {
    job_id: String,
    state: String,
    retry_of_job_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogView {
    id: String,
    stage: String,
    level: String,
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderArtifactView {
    id: String,
    job_id: String,
    project_id: String,
    file_path: String,
    duration_secs: f64,
    file_size_bytes: i64,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFileUrlResult {
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRenderArtifactResult {
    opened: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenRenderArtifactMode {
    OpenFile,
    RevealInFolder,
}

struct ResolvedArtifact {
    id: String,
    job_id: String,
    canonical_path: PathBuf,
}

const URI_COMPONENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'!')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

#[tauri::command]
pub fn submit_prompt_job(
    project_id: String,
    provider_id: String,
    prompt_text: String,
    state: State<'_, AppState>,
) -> AppResponse<SubmitPromptJobResult> {
    tauri::async_runtime::block_on(submit_prompt_job_inner(
        project_id,
        provider_id,
        prompt_text,
        state.inner(),
    ))
}

async fn submit_prompt_job_inner(
    project_id: String,
    provider_id: String,
    prompt_text: String,
    state: &AppState,
) -> AppResponse<SubmitPromptJobResult> {
    let prompt_text = prompt_text.trim();
    if prompt_text.is_empty() {
        return AppResponse::err(AppError::new(E_VALIDATION, "提示词不能为空", false));
    }

    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    if let Err(error) = ensure_project_exists(&pool, &project_id).await {
        return AppResponse::err(error);
    }
    if let Err(error) = ensure_provider_exists(&pool, &provider_id).await {
        return AppResponse::err(error);
    }

    let job_id = format!("job_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();

    if let Err(error) = sqlx::query(
        "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) \
         VALUES (?, ?, ?, ?, 'queued', NULL, NULL, NULL, NULL, ?, NULL, NULL)",
    )
    .bind(&job_id)
    .bind(&project_id)
    .bind(&provider_id)
    .bind(prompt_text)
    .bind(&now)
    .execute(&pool)
    .await
    {
        return AppResponse::err(AppError::new(
            E_DB,
            format!("无法创建任务: {error}"),
            false,
        ));
    }

    logging::write_job_log(
        Some(&pool),
        Some(&workspace_root),
        &job_id,
        "queue",
        "info",
        "job created and queued",
    )
    .await;

    if let Err(error) = queue::enqueue(state, job_id.clone()).await {
        let _ = sqlx::query("DELETE FROM prompt_jobs WHERE id = ?")
            .bind(&job_id)
            .execute(&pool)
            .await;
        return AppResponse::err(error);
    }

    AppResponse::ok(SubmitPromptJobResult {
        job_id,
        state: "queued".to_string(),
    })
}

#[tauri::command]
pub fn get_job(job_id: String, state: State<'_, AppState>) -> AppResponse<PromptJobView> {
    tauri::async_runtime::block_on(get_job_inner(job_id, state.inner()))
}

async fn get_job_inner(job_id: String, state: &AppState) -> AppResponse<PromptJobView> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };

    let row = match sqlx::query(
        "SELECT id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at \
         FROM prompt_jobs WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&job_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return AppResponse::err(AppError::new(E_NOT_FOUND, "任务不存在", false)),
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取任务: {error}"),
                false,
            ));
        }
    };

    AppResponse::ok(map_job_row(row))
}

#[tauri::command]
pub fn list_project_jobs(
    project_id: String,
    state: State<'_, AppState>,
) -> AppResponse<Vec<PromptJobView>> {
    tauri::async_runtime::block_on(list_project_jobs_inner(project_id, state.inner()))
}

async fn list_project_jobs_inner(
    project_id: String,
    state: &AppState,
) -> AppResponse<Vec<PromptJobView>> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::ok(Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at \
         FROM prompt_jobs WHERE project_id = ? AND deleted_at IS NULL ORDER BY created_at DESC",
    )
    .bind(&project_id)
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取项目任务列表: {error}"),
                false,
            ));
        }
    };

    AppResponse::ok(rows.into_iter().map(map_job_row).collect())
}

#[tauri::command]
pub fn cancel_job(job_id: String, state: State<'_, AppState>) -> AppResponse<CancelJobResult> {
    tauri::async_runtime::block_on(cancel_job_inner(job_id, state.inner()))
}

async fn cancel_job_inner(job_id: String, state: &AppState) -> AppResponse<CancelJobResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    let current_state = match load_job_state(&pool, &job_id).await {
        Ok(state_value) => state_value,
        Err(error) => return AppResponse::err(error),
    };

    match current_state.as_str() {
        "queued" => {
            let now = Utc::now().to_rfc3339();
            if let Err(error) = sqlx::query(
                "UPDATE prompt_jobs \
                 SET state = 'cancelled', error_code = ?, error_summary = ?, suggestion = ?, finished_at = ? \
                 WHERE id = ?",
            )
            .bind(E_CANCELLED)
            .bind("任务已取消")
            .bind("如需继续生成，请手动重试任务。")
            .bind(&now)
            .bind(&job_id)
            .execute(&pool)
            .await
            {
                return AppResponse::err(AppError::new(
                    E_DB,
                    format!("无法取消任务: {error}"),
                    false,
                ));
            }

            logging::write_job_log(
                Some(&pool),
                Some(&workspace_root),
                &job_id,
                "user_action",
                "info",
                "queued job cancelled by user",
            )
            .await;

            AppResponse::ok(CancelJobResult {
                job_id,
                state: "cancelled".to_string(),
            })
        }
        "running" => {
            let Some((running_job_id, cancel_flag)) = state.get_running_job().await else {
                return AppResponse::err(AppError::new(
                    E_CANCEL_FAILED,
                    "任务正在运行，但当前没有可取消的进程句柄",
                    false,
                ));
            };

            if running_job_id != job_id {
                return AppResponse::err(AppError::new(
                    E_CANCEL_FAILED,
                    "当前运行中的任务与请求取消的任务不匹配",
                    false,
                ));
            }

            cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);

            if let Err(error) = sqlx::query(
                "UPDATE prompt_jobs \
                 SET state = 'cancelled', error_code = ?, error_summary = ?, suggestion = ?, finished_at = ? \
                 WHERE id = ?",
            )
            .bind(E_CANCELLED)
            .bind("任务已取消")
            .bind("任务正在停止中；如需继续生成，请稍后手动重试。")
            .bind(Utc::now().to_rfc3339())
            .bind(&job_id)
            .execute(&pool)
            .await
            {
                return AppResponse::err(AppError::new(
                    E_DB,
                    format!("无法标记取消状态: {error}"),
                    false,
                ));
            }

            logging::write_job_log(
                Some(&pool),
                Some(&workspace_root),
                &job_id,
                "user_action",
                "info",
                "running job cancellation requested by user",
            )
            .await;

            AppResponse::ok(CancelJobResult {
                job_id,
                state: "cancelled".to_string(),
            })
        }
        _ => AppResponse::err(AppError::new(
            E_JOB_NOT_CANCELLABLE,
            "当前任务状态不允许取消",
            false,
        )),
    }
}

#[tauri::command]
pub fn delete_job(job_id: String, state: State<'_, AppState>) -> AppResponse<DeleteJobResult> {
    tauri::async_runtime::block_on(delete_job_inner(job_id, state.inner()))
}

async fn delete_job_inner(job_id: String, state: &AppState) -> AppResponse<DeleteJobResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    let current_state = match load_job_state(&pool, &job_id).await {
        Ok(state_value) => state_value,
        Err(error) => return AppResponse::err(error),
    };

    if matches!(current_state.as_str(), "queued" | "running") {
        return AppResponse::err(AppError::new(
            E_JOB_NOT_DELETABLE,
            "queued or running jobs cannot be deleted",
            false,
        ));
    }

    let now = Utc::now().to_rfc3339();
    let result = match sqlx::query(
        "UPDATE prompt_jobs SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(&job_id)
    .execute(&pool)
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("failed to delete job: {error}"),
                false,
            ));
        }
    };

    if result.rows_affected() == 0 {
        return AppResponse::err(AppError::new(E_NOT_FOUND, "job not found", false));
    }

    logging::write_job_log(
        Some(&pool),
        Some(&workspace_root),
        &job_id,
        "user_action",
        "info",
        "job hidden by user",
    )
    .await;

    AppResponse::ok(DeleteJobResult { deleted: true })
}

#[tauri::command]
pub fn retry_job(job_id: String, state: State<'_, AppState>) -> AppResponse<RetryJobResult> {
    tauri::async_runtime::block_on(retry_job_inner(job_id, state.inner()))
}

async fn retry_job_inner(job_id: String, state: &AppState) -> AppResponse<RetryJobResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    let row = match sqlx::query(
        "SELECT project_id, provider_id, prompt_text, state FROM prompt_jobs WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&job_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return AppResponse::err(AppError::new(E_NOT_FOUND, "任务不存在", false)),
        Err(error) => {
            return AppResponse::err(AppError::new(E_DB, format!("无法读取任务: {error}"), false));
        }
    };

    let original_state: String = row.get("state");
    if !matches!(original_state.as_str(), "failed" | "cancelled") {
        return AppResponse::err(AppError::new(
            E_JOB_NOT_RETRYABLE,
            "只有 failed 或 cancelled 任务允许手动重试",
            false,
        ));
    }

    let new_job_id = format!("job_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let project_id: String = row.get("project_id");
    let provider_id: String = row.get("provider_id");
    let prompt_text: String = row.get("prompt_text");

    if let Err(error) = ensure_provider_reference_exists(&pool, &provider_id).await {
        return AppResponse::err(AppError::new(
            error.code,
            "原任务引用的 Provider 不存在或已删除，请重新选择 Provider 后提交新任务",
            false,
        ));
    }

    if let Err(error) = sqlx::query(
        "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) \
         VALUES (?, ?, ?, ?, 'queued', NULL, NULL, NULL, ?, ?, NULL, NULL)",
    )
    .bind(&new_job_id)
    .bind(&project_id)
    .bind(&provider_id)
    .bind(&prompt_text)
    .bind(&job_id)
    .bind(&now)
    .execute(&pool)
    .await
    {
        return AppResponse::err(AppError::new(
            E_DB,
            format!("无法创建重试任务: {error}"),
            false,
        ));
    }

    logging::write_job_log(
        Some(&pool),
        Some(&workspace_root),
        &new_job_id,
        "user_action",
        "info",
        &format!("retry job created from {job_id}"),
    )
    .await;

    if let Err(error) = queue::enqueue(state, new_job_id.clone()).await {
        let _ = sqlx::query("DELETE FROM prompt_jobs WHERE id = ?")
            .bind(&new_job_id)
            .execute(&pool)
            .await;
        return AppResponse::err(error);
    }

    AppResponse::ok(RetryJobResult {
        job_id: new_job_id,
        state: "queued".to_string(),
        retry_of_job_id: job_id,
    })
}

#[tauri::command]
pub fn get_job_logs(job_id: String, state: State<'_, AppState>) -> AppResponse<Vec<JobLogView>> {
    tauri::async_runtime::block_on(get_job_logs_inner(job_id, state.inner()))
}

async fn get_job_logs_inner(job_id: String, state: &AppState) -> AppResponse<Vec<JobLogView>> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::ok(Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT id, stage, level, message, timestamp FROM job_logs WHERE job_id = ? ORDER BY timestamp ASC",
    )
    .bind(&job_id)
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取任务日志: {error}"),
                false,
            ));
        }
    };

    AppResponse::ok(
        rows.into_iter()
            .map(|row| JobLogView {
                id: row.get("id"),
                stage: row.get("stage"),
                level: row.get("level"),
                message: row.get("message"),
                timestamp: row.get("timestamp"),
            })
            .collect(),
    )
}

#[tauri::command]
pub fn get_render_artifact(
    job_id: String,
    state: State<'_, AppState>,
) -> AppResponse<RenderArtifactView> {
    tauri::async_runtime::block_on(get_render_artifact_inner(job_id, state.inner()))
}

async fn get_render_artifact_inner(
    job_id: String,
    state: &AppState,
) -> AppResponse<RenderArtifactView> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };

    let row = match sqlx::query(
        "SELECT id, job_id, project_id, file_path, duration_secs, file_size_bytes, created_at \
         FROM render_artifacts WHERE job_id = ?",
    )
    .bind(&job_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return AppResponse::err(AppError::new(
                E_NOT_FOUND,
                "当前任务尚未生成可用产物",
                false,
            ));
        }
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取渲染产物: {error}"),
                false,
            ));
        }
    };

    AppResponse::ok(RenderArtifactView {
        id: row.get("id"),
        job_id: row.get("job_id"),
        project_id: row.get("project_id"),
        file_path: row.get("file_path"),
        duration_secs: row.get("duration_secs"),
        file_size_bytes: row.get("file_size_bytes"),
        created_at: row.get("created_at"),
    })
}

#[tauri::command]
pub fn get_video_file_url(
    artifact_id: String,
    state: State<'_, AppState>,
) -> AppResponse<VideoFileUrlResult> {
    tauri::async_runtime::block_on(get_video_file_url_inner(artifact_id, state.inner()))
}

async fn get_video_file_url_inner(
    artifact_id: String,
    state: &AppState,
) -> AppResponse<VideoFileUrlResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    let resolved = match resolve_artifact_by_id(
        &pool,
        &workspace_root,
        &artifact_id,
        "当前任务尚未生成可预览的视频",
    )
    .await
    {
        Ok(resolved) => resolved,
        Err(error) => return AppResponse::err(error),
    };

    AppResponse::ok(VideoFileUrlResult {
        url: build_tauri_asset_url(&resolved.canonical_path),
    })
}

#[tauri::command]
pub fn open_render_artifact(
    artifact_id: String,
    mode: String,
    state: State<'_, AppState>,
) -> AppResponse<OpenRenderArtifactResult> {
    tauri::async_runtime::block_on(open_render_artifact_inner(artifact_id, mode, state.inner()))
}

async fn open_render_artifact_inner(
    artifact_id: String,
    mode: String,
    state: &AppState,
) -> AppResponse<OpenRenderArtifactResult> {
    open_render_artifact_inner_with_launcher(artifact_id, mode, state, launch_artifact_path).await
}

async fn open_render_artifact_inner_with_launcher<F>(
    artifact_id: String,
    mode: String,
    state: &AppState,
    launcher: F,
) -> AppResponse<OpenRenderArtifactResult>
where
    F: Fn(&Path, OpenRenderArtifactMode) -> Result<(), String>,
{
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(workspace_not_ready());
    };
    let Some(workspace_root) = workspace_root_from_state(state).await else {
        return AppResponse::err(workspace_not_ready());
    };

    let open_mode = match parse_open_mode(&mode) {
        Ok(mode) => mode,
        Err(error) => return AppResponse::err(error),
    };

    let resolved = match resolve_artifact_by_id(
        &pool,
        &workspace_root,
        &artifact_id,
        "渲染产物不存在或已失效",
    )
    .await
    {
        Ok(resolved) => resolved,
        Err(error) => return AppResponse::err(error),
    };

    if let Err(error) = launcher(&resolved.canonical_path, open_mode) {
        return AppResponse::err(AppError::new(E_IO, error, false));
    }

    logging::write_job_log(
        Some(&pool),
        Some(&workspace_root),
        &resolved.job_id,
        "user_action",
        "info",
        &format!("artifact {} opened via {}", resolved.id, mode),
    )
    .await;

    AppResponse::ok(OpenRenderArtifactResult { opened: true })
}

async fn ensure_project_exists(pool: &SqlitePool, project_id: &str) -> Result<(), AppError> {
    ensure_exists(
        pool,
        "SELECT 1 FROM projects WHERE id = ? AND deleted_at IS NULL LIMIT 1",
        project_id,
        "项目不存在",
    )
    .await
}

async fn ensure_provider_exists(pool: &SqlitePool, provider_id: &str) -> Result<(), AppError> {
    ensure_exists(
        pool,
        "SELECT 1 FROM provider_configs WHERE id = ? AND deleted_at IS NULL LIMIT 1",
        provider_id,
        "Provider 不存在或已删除",
    )
    .await
}

async fn ensure_provider_reference_exists(
    pool: &SqlitePool,
    provider_id: &str,
) -> Result<(), AppError> {
    ensure_exists(
        pool,
        "SELECT 1 FROM provider_configs WHERE id = ? LIMIT 1",
        provider_id,
        "原任务引用的 Provider 不存在",
    )
    .await
}

async fn ensure_exists(
    pool: &SqlitePool,
    statement: &str,
    id: &str,
    not_found_message: &str,
) -> Result<(), AppError> {
    let exists = sqlx::query(statement)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|error| AppError::new(E_DB, format!("无法校验引用记录: {error}"), false))?
        .is_some();

    if exists {
        Ok(())
    } else {
        Err(AppError::new(E_NOT_FOUND, not_found_message, false))
    }
}

async fn load_job_state(pool: &SqlitePool, job_id: &str) -> Result<String, AppError> {
    let row = sqlx::query("SELECT state FROM prompt_jobs WHERE id = ? AND deleted_at IS NULL")
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| AppError::new(E_DB, format!("无法读取任务状态: {error}"), false))?;

    let Some(row) = row else {
        return Err(AppError::new(E_NOT_FOUND, "任务不存在", false));
    };

    Ok(row.get("state"))
}

async fn workspace_root_from_state(state: &AppState) -> Option<PathBuf> {
    state.get_workspace_path().await.map(PathBuf::from)
}

fn parse_open_mode(mode: &str) -> Result<OpenRenderArtifactMode, AppError> {
    match mode {
        "open_file" => Ok(OpenRenderArtifactMode::OpenFile),
        "reveal_in_folder" => Ok(OpenRenderArtifactMode::RevealInFolder),
        _ => Err(AppError::new(
            E_VALIDATION,
            "打开渲染产物时使用了不支持的模式",
            false,
        )),
    }
}

async fn resolve_artifact_by_id(
    pool: &SqlitePool,
    workspace_root: &Path,
    artifact_id: &str,
    not_found_message: &str,
) -> Result<ResolvedArtifact, AppError> {
    let row = sqlx::query("SELECT id, job_id, file_path FROM render_artifacts WHERE id = ?")
        .bind(artifact_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| AppError::new(E_DB, format!("无法读取渲染产物: {error}"), false))?;

    let Some(row) = row else {
        return Err(AppError::new(E_NOT_FOUND, not_found_message, false));
    };

    let relative_path: String = row.get("file_path");
    let canonical_path = validate_artifact_path(workspace_root, &relative_path).await?;

    Ok(ResolvedArtifact {
        id: row.get("id"),
        job_id: row.get("job_id"),
        canonical_path,
    })
}

async fn validate_artifact_path(
    workspace_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    let canonical_workspace = tokio::fs::canonicalize(workspace_root)
        .await
        .map_err(|error| {
            AppError::new(
                E_WORKSPACE_INVALID,
                format!("无法确认工作区路径: {error}"),
                false,
            )
        })?;
    let canonical_artifact = tokio::fs::canonicalize(workspace_root.join(relative_path))
        .await
        .map_err(|_| AppError::new(E_ARTIFACT_INVALID, "渲染产物文件不存在或已损坏", false))?;

    if !canonical_artifact.starts_with(&canonical_workspace) {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "渲染产物路径超出当前工作区范围",
            false,
        ));
    }

    Ok(canonical_artifact)
}

fn build_tauri_asset_url(file_path: &Path) -> String {
    let encoded_path =
        utf8_percent_encode(&file_path.to_string_lossy(), URI_COMPONENT_ENCODE_SET).to_string();

    if cfg!(target_os = "windows") {
        format!("http://asset.localhost/{encoded_path}")
    } else {
        format!("asset://localhost/{encoded_path}")
    }
}

fn launch_artifact_path(path: &Path, mode: OpenRenderArtifactMode) -> Result<(), String> {
    let status = match mode {
        OpenRenderArtifactMode::OpenFile => open_file_with_default_app(path),
        OpenRenderArtifactMode::RevealInFolder => reveal_path_in_folder(path),
    }?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("系统打开命令执行失败，退出码: {:?}", status.code()))
    }
}

fn open_file_with_default_app(path: &Path) -> Result<std::process::ExitStatus, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .status()
            .map_err(|error| format!("无法打开渲染文件: {error}"))
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .status()
            .map_err(|error| format!("无法打开渲染文件: {error}"))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|error| format!("无法打开渲染文件: {error}"))
    }
}

fn reveal_path_in_folder(path: &Path) -> Result<std::process::ExitStatus, String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .status()
            .map_err(|error| format!("无法在资源管理器中定位渲染文件: {error}"))
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|error| format!("无法在 Finder 中定位渲染文件: {error}"))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = path.parent().unwrap_or(path);
        Command::new("xdg-open")
            .arg(target)
            .status()
            .map_err(|error| format!("无法在文件管理器中定位渲染文件: {error}"))
    }
}

fn workspace_not_ready() -> AppError {
    AppError::new(E_WORKSPACE_INVALID, "工作区尚未初始化", false)
}

fn map_job_row(row: sqlx::sqlite::SqliteRow) -> PromptJobView {
    PromptJobView {
        id: row.get("id"),
        project_id: row.get("project_id"),
        provider_id: row.get("provider_id"),
        prompt_text: row.get("prompt_text"),
        state: row.get("state"),
        error_code: row.try_get("error_code").unwrap_or(None),
        error_summary: row.try_get("error_summary").unwrap_or(None),
        suggestion: row.try_get("suggestion").unwrap_or(None),
        retry_of_job_id: row.try_get("retry_of_job_id").unwrap_or(None),
        created_at: row.get("created_at"),
        started_at: row.try_get("started_at").unwrap_or(None),
        finished_at: row.try_get("finished_at").unwrap_or(None),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    use chrono::Utc;
    use sqlx::{Row, SqlitePool};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    use super::*;
    use crate::{
        services::{db, logging, workspace},
        types::error_codes::{E_ARTIFACT_INVALID, E_CANCELLED},
    };

    #[tokio::test]
    async fn retry_job_allows_soft_deleted_provider_reference() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "failed").await;

        soft_delete_provider(&pool, &provider_id).await;

        let response = retry_job_inner(job_id.clone(), &state).await;

        assert!(response.ok, "retry response should succeed");
        let data = response.data.unwrap();
        assert_eq!(data.retry_of_job_id, job_id);

        let row =
            sqlx::query("SELECT provider_id, retry_of_job_id, state FROM prompt_jobs WHERE id = ?")
                .bind(&data.job_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let new_provider_id: String = row.get("provider_id");
        let retry_of_job_id: Option<String> = row.get("retry_of_job_id");
        let new_state: String = row.get("state");

        assert_eq!(new_provider_id, provider_id);
        assert_eq!(retry_of_job_id.as_deref(), Some(job_id.as_str()));
        assert_eq!(new_state, "queued");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn delete_job_soft_deletes_terminal_job_and_hides_it_from_reads() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let failed_job_id = insert_prompt_job(&pool, &project_id, &provider_id, "failed").await;
        let succeeded_job_id =
            insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;

        let response = delete_job_inner(failed_job_id.clone(), &state).await;

        assert!(response.ok, "failed job should be deletable");
        assert!(response.data.unwrap().deleted);

        let row = sqlx::query("SELECT deleted_at FROM prompt_jobs WHERE id = ?")
            .bind(&failed_job_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let deleted_at: Option<String> = row.get("deleted_at");
        assert!(deleted_at.is_some());

        let list_response = list_project_jobs_inner(project_id, &state).await;
        assert!(list_response.ok);
        let visible_jobs = list_response.data.unwrap();
        assert!(visible_jobs.iter().all(|job| job.id != failed_job_id));
        assert!(visible_jobs.iter().any(|job| job.id == succeeded_job_id));

        let get_response = get_job_inner(failed_job_id, &state).await;
        assert!(
            !get_response.ok,
            "deleted job should be hidden from get_job"
        );
        assert_eq!(get_response.error.unwrap().code, E_NOT_FOUND);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn delete_job_rejects_live_jobs() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let queued_job_id = insert_prompt_job(&pool, &project_id, &provider_id, "queued").await;
        let running_job_id = insert_prompt_job(&pool, &project_id, &provider_id, "running").await;

        let queued_response = delete_job_inner(queued_job_id, &state).await;
        let running_response = delete_job_inner(running_job_id, &state).await;

        assert!(!queued_response.ok, "queued job should not be deletable");
        assert_eq!(queued_response.error.unwrap().code, E_JOB_NOT_DELETABLE);
        assert!(!running_response.ok, "running job should not be deletable");
        assert_eq!(running_response.error.unwrap().code, E_JOB_NOT_DELETABLE);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn get_video_file_url_returns_asset_url() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;
        let relative_path = PathBuf::from("artifacts")
            .join(&project_id)
            .join(&job_id)
            .join("preview.mp4");
        let absolute_path = workspace_root.join(&relative_path);

        tokio::fs::create_dir_all(absolute_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&absolute_path, b"fake-mp4").await.unwrap();
        let artifact_id = insert_render_artifact(&pool, &job_id, &project_id, &relative_path).await;

        let response = get_video_file_url_inner(artifact_id, &state).await;

        assert!(
            response.ok,
            "video file url should resolve inside workspace"
        );
        let data = response.data.unwrap();
        let canonical = tokio::fs::canonicalize(&absolute_path).await.unwrap();
        assert_eq!(data.url, build_tauri_asset_url(&canonical));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn get_video_file_url_rejects_paths_outside_workspace() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;
        let outside_path = workspace_root
            .parent()
            .unwrap_or(workspace_root.as_path())
            .join(format!("outside-video-{}.mp4", Uuid::new_v4()));
        let relative_escape_path = PathBuf::from("..").join(outside_path.file_name().unwrap());

        tokio::fs::write(&outside_path, b"fake-mp4").await.unwrap();
        let artifact_id =
            insert_render_artifact(&pool, &job_id, &project_id, &relative_escape_path).await;

        let response = get_video_file_url_inner(artifact_id, &state).await;

        assert!(!response.ok, "path traversal should be rejected");
        assert_eq!(response.error.unwrap().code, E_ARTIFACT_INVALID);

        let _ = tokio::fs::remove_file(outside_path).await;
        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn open_render_artifact_validates_path_and_invokes_launcher() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;
        let relative_path = PathBuf::from("artifacts")
            .join(&project_id)
            .join(&job_id)
            .join("preview.mp4");
        let absolute_path = workspace_root.join(&relative_path);

        tokio::fs::create_dir_all(absolute_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&absolute_path, b"fake-mp4").await.unwrap();
        let artifact_id = insert_render_artifact(&pool, &job_id, &project_id, &relative_path).await;

        let captured = std::sync::Mutex::new(None::<(PathBuf, OpenRenderArtifactMode)>);
        let response = open_render_artifact_inner_with_launcher(
            artifact_id,
            "reveal_in_folder".to_string(),
            &state,
            |path, mode| {
                *captured.lock().unwrap() = Some((path.to_path_buf(), mode));
                Ok(())
            },
        )
        .await;

        assert!(
            response.ok,
            "artifact opener should succeed for valid artifact"
        );
        let captured = captured.lock().unwrap().clone().unwrap();
        assert_eq!(
            captured.0,
            tokio::fs::canonicalize(&absolute_path).await.unwrap()
        );
        assert_eq!(captured.1, OpenRenderArtifactMode::RevealInFolder);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn open_render_artifact_rejects_paths_outside_workspace() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;
        let outside_path = workspace_root
            .parent()
            .unwrap_or(workspace_root.as_path())
            .join(format!("outside-artifact-{}.mp4", Uuid::new_v4()));
        let relative_escape_path = PathBuf::from("..").join(outside_path.file_name().unwrap());

        tokio::fs::write(&outside_path, b"fake-mp4").await.unwrap();
        let artifact_id =
            insert_render_artifact(&pool, &job_id, &project_id, &relative_escape_path).await;

        let response = open_render_artifact_inner_with_launcher(
            artifact_id,
            "open_file".to_string(),
            &state,
            |_, _| panic!("launcher should not be called for invalid artifact path"),
        )
        .await;

        assert!(!response.ok, "artifact opener should reject path traversal");
        assert_eq!(response.error.unwrap().code, E_ARTIFACT_INVALID);

        let _ = tokio::fs::remove_file(outside_path).await;
        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn command_flow_supports_submit_cancel_retry_preview_and_open() {
        let (state, workspace_root, mut queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;

        let submit = submit_prompt_job_inner(
            project_id.clone(),
            provider_id.clone(),
            "用动画解释二次方程求根公式".to_string(),
            &state,
        )
        .await;
        assert!(submit.ok, "submit should succeed in integration flow");
        let first_job_id = submit.data.unwrap().job_id;
        assert_eq!(queue_rx.recv().await.unwrap(), first_job_id);

        let queued_job = get_job_inner(first_job_id.clone(), &state).await;
        assert!(queued_job.ok);
        assert_eq!(queued_job.data.unwrap().state, "queued");

        let cancel = cancel_job_inner(first_job_id.clone(), &state).await;
        assert!(
            cancel.ok,
            "queued job should be cancellable in integration flow"
        );

        let retry = retry_job_inner(first_job_id.clone(), &state).await;
        assert!(
            retry.ok,
            "cancelled job should be retryable in integration flow"
        );
        let retried_job_id = retry.data.unwrap().job_id;
        assert_eq!(queue_rx.recv().await.unwrap(), retried_job_id);

        sqlx::query("UPDATE prompt_jobs SET state = 'succeeded', finished_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(&retried_job_id)
            .execute(&pool)
            .await
            .unwrap();

        let relative_path = PathBuf::from("artifacts")
            .join(&project_id)
            .join(&retried_job_id)
            .join("output.mp4");
        let absolute_path = workspace_root.join(&relative_path);
        tokio::fs::create_dir_all(absolute_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&absolute_path, b"fake-mp4").await.unwrap();
        let artifact_id =
            insert_render_artifact(&pool, &retried_job_id, &project_id, &relative_path).await;

        let logs = get_job_logs_inner(first_job_id.clone(), &state).await;
        assert!(logs.ok, "logs should be readable for cancelled job");
        assert!(logs
            .data
            .unwrap()
            .iter()
            .any(|entry| entry.message == "queued job cancelled by user"));

        let artifact = get_render_artifact_inner(retried_job_id.clone(), &state).await;
        assert!(
            artifact.ok,
            "render artifact should be readable for succeeded retry job"
        );
        let artifact = artifact.data.unwrap();

        let preview = get_video_file_url_inner(artifact.id.clone(), &state).await;
        assert!(preview.ok, "preview url should be generated for artifact");
        assert_eq!(
            preview.data.unwrap().url,
            build_tauri_asset_url(&tokio::fs::canonicalize(&absolute_path).await.unwrap())
        );

        let open = open_render_artifact_inner_with_launcher(
            artifact_id,
            "reveal_in_folder".to_string(),
            &state,
            |_, _| Ok(()),
        )
        .await;
        assert!(
            open.ok,
            "artifact opener should succeed in integration flow"
        );

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn cancel_job_marks_queued_job_cancelled_and_exposes_log_entries() {
        let (state, workspace_root, _queue_rx) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = insert_provider(&pool).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "queued").await;

        logging::write_job_log(
            Some(&pool),
            Some(&workspace_root),
            &job_id,
            "queue",
            "info",
            "job created and queued",
        )
        .await;

        let response = cancel_job_inner(job_id.clone(), &state).await;

        assert!(response.ok, "queued job should be cancellable");

        let row =
            sqlx::query("SELECT state, error_code, finished_at FROM prompt_jobs WHERE id = ?")
                .bind(&job_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let state_value: String = row.get("state");
        let error_code: Option<String> = row.get("error_code");
        let finished_at: Option<String> = row.get("finished_at");

        assert_eq!(state_value, "cancelled");
        assert_eq!(error_code.as_deref(), Some(E_CANCELLED));
        assert!(finished_at.is_some());

        let logs_response = get_job_logs_inner(job_id, &state).await;

        assert!(
            logs_response.ok,
            "job logs should be queryable after cancellation"
        );
        let logs = logs_response.data.unwrap();
        assert!(logs
            .iter()
            .any(|entry| entry.message == "job created and queued"));
        assert!(logs
            .iter()
            .any(|entry| entry.message == "queued job cancelled by user"));

        cleanup(workspace_root).await;
    }

    async fn setup_test_state() -> (AppState, PathBuf, mpsc::UnboundedReceiver<String>) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-job-tests-{}", Uuid::new_v4()));
        workspace::create_standard_dirs(&workspace_root)
            .await
            .unwrap();

        let pool = db::open_or_create(&workspace_root.join("db").join("app.sqlite"))
            .await
            .unwrap();
        let state = AppState::default();
        let (queue_tx, queue_rx) = mpsc::unbounded_channel();

        state
            .set_workspace(workspace_root.to_string_lossy().into_owned(), pool)
            .await;
        state.set_queue_sender(queue_tx).await;

        (state, workspace_root, queue_rx)
    }

    async fn insert_provider(pool: &SqlitePool) -> String {
        let provider_id = format!("provider_{}", Uuid::new_v4());
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
        .execute(pool)
        .await
        .unwrap();

        provider_id
    }

    async fn soft_delete_provider(pool: &SqlitePool, provider_id: &str) {
        let now = Utc::now().to_rfc3339();

        sqlx::query("UPDATE provider_configs SET deleted_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(provider_id)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn insert_project(pool: &SqlitePool) -> String {
        let project_id = format!("project_{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO projects (id, name, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, NULL)",
        )
        .bind(&project_id)
        .bind("Test Project")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        project_id
    }

    async fn insert_prompt_job(
        pool: &SqlitePool,
        project_id: &str,
        provider_id: &str,
        state: &str,
    ) -> String {
        let job_id = format!("job_{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, ?, NULL, NULL)",
        )
        .bind(&job_id)
        .bind(project_id)
        .bind(provider_id)
        .bind("Explain quadratic formula")
        .bind(state)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        job_id
    }

    async fn insert_render_artifact(
        pool: &SqlitePool,
        job_id: &str,
        project_id: &str,
        relative_path: &Path,
    ) -> String {
        let artifact_id = format!("artifact_{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO render_artifacts (id, job_id, project_id, file_path, duration_secs, file_size_bytes, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&artifact_id)
        .bind(job_id)
        .bind(project_id)
        .bind(relative_path.to_string_lossy().into_owned())
        .bind(1.2_f64)
        .bind(1024_i64)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        artifact_id
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
