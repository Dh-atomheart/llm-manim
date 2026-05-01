use std::{
    future::Future,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    services::{artifact, logging, orchestrator, render},
    state::AppState,
    types::{
        error_codes::{E_CANCELLED, E_IO, E_NOT_FOUND, E_RENDER_FAIL, E_RENDER_TIMEOUT},
        response::AppError,
    },
};

pub async fn enqueue(state: &AppState, job_id: String) -> Result<(), AppError> {
    let Some(queue_tx) = state.get_queue_sender().await else {
        return Err(AppError::new(E_IO, "任务队列尚未初始化", true));
    };

    queue_tx
        .send(job_id)
        .map_err(|_| AppError::new(E_IO, "任务队列当前不可用", true))
}

pub async fn run_queue_worker<F, Fut>(mut queue_rx: UnboundedReceiver<String>, mut process: F)
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = ()>,
{
    while let Some(job_id) = queue_rx.recv().await {
        process(job_id).await;
    }
}

pub async fn recover_running_jobs(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let Some(pool) = state.get_db().await else {
        return;
    };
    let Some(workspace_path) = state.get_workspace_path().await else {
        return;
    };
    if !prompt_jobs_table_exists(&pool).await {
        return;
    }

    let workspace_root = PathBuf::from(workspace_path);
    let rows = match sqlx::query("SELECT id FROM prompt_jobs WHERE state = 'running'")
        .fetch_all(&pool)
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("failed to recover running jobs: {error}");
            return;
        }
    };

    for row in rows {
        let job_id: String = row.get("id");
        let error = AppError::new(
            E_RENDER_FAIL,
            "应用重启时发现未完成的运行中任务，已标记为失败",
            true,
        );
        let _ = set_job_failed(&pool, &workspace_root, &job_id, &error).await;
    }
}

pub async fn requeue_queued_jobs(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let Some(pool) = state.get_db().await else {
        return;
    };
    if !prompt_jobs_table_exists(&pool).await {
        return;
    }

    let rows = match sqlx::query(
        "SELECT id FROM prompt_jobs WHERE state = 'queued' ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("failed to requeue queued jobs: {error}");
            return;
        }
    };

    for row in rows {
        let job_id: String = row.get("id");
        if let Err(error) = enqueue(state.inner(), job_id).await {
            eprintln!("failed to enqueue restored job: {}", error.message);
            break;
        }
    }
}

pub async fn process_job(app_handle: AppHandle, job_id: String) {
    let state = app_handle.state::<AppState>();
    let Some(pool) = state.get_db().await else {
        return;
    };
    let Some(workspace_path) = state.get_workspace_path().await else {
        return;
    };
    let workspace_root = PathBuf::from(workspace_path);

    if !claim_job_running(&pool, &job_id).await {
        return;
    }

    let Ok(project_id) = load_project_id(&pool, &job_id).await else {
        return;
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    state
        .set_running_job(job_id.clone(), cancel_flag.clone())
        .await;

    logging::write_job_log(
        Some(&pool),
        Some(&workspace_root),
        &job_id,
        "queue",
        "info",
        "job dequeued and entered running state",
    )
    .await;

    let outcome = process_job_inner(
        &pool,
        &workspace_root,
        &job_id,
        &project_id,
        cancel_flag.clone(),
    )
    .await;

    if cancel_flag.load(Ordering::Relaxed) || is_job_cancelled(&pool, &job_id).await {
        let _ = set_job_cancelled(&pool, &workspace_root, &job_id).await;
        state.clear_running_job().await;
        return;
    }

    match outcome {
        Ok(artifact_info) => {
            let _ = mark_job_succeeded(&pool, &workspace_root, &job_id, &project_id, artifact_info)
                .await;
        }
        Err(error) if error.code == E_CANCELLED => {
            let _ = set_job_cancelled(&pool, &workspace_root, &job_id).await;
        }
        Err(error) => {
            let _ = set_job_failed(&pool, &workspace_root, &job_id, &error).await;
        }
    }

    state.clear_running_job().await;
}

async fn process_job_inner(
    pool: &SqlitePool,
    workspace_root: &PathBuf,
    job_id: &str,
    project_id: &str,
    cancel_flag: Arc<AtomicBool>,
) -> Result<artifact::ArtifactInfo, AppError> {
    let orchestration =
        orchestrator::run_orchestration(pool, workspace_root, job_id, cancel_flag.clone()).await?;

    render::write_scene_file(workspace_root, job_id, &orchestration.code).await?;
    render::write_manim_cfg(workspace_root, job_id).await?;
    render::run_manim(
        Some(pool),
        workspace_root,
        job_id,
        project_id,
        &orchestration.scene_name,
        cancel_flag,
    )
    .await?;

    artifact::check_artifact(workspace_root, project_id, job_id).await
}

async fn prompt_jobs_table_exists(pool: &SqlitePool) -> bool {
    sqlx::query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'prompt_jobs' LIMIT 1")
        .fetch_optional(pool)
        .await
        .map(|row| row.is_some())
        .unwrap_or(false)
}

async fn claim_job_running(pool: &SqlitePool, job_id: &str) -> bool {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE prompt_jobs \
         SET state = 'running', started_at = ?, finished_at = NULL, error_code = NULL, error_summary = NULL, suggestion = NULL \
         WHERE id = ? AND state = 'queued'",
    )
    .bind(now)
    .bind(job_id)
    .execute(pool)
    .await
    .map(|result| result.rows_affected() > 0)
    .unwrap_or(false)
}

async fn load_project_id(pool: &SqlitePool, job_id: &str) -> Result<String, AppError> {
    let row = sqlx::query("SELECT project_id FROM prompt_jobs WHERE id = ?")
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| AppError::new(E_NOT_FOUND, format!("无法读取任务项目: {error}"), false))?;

    let Some(row) = row else {
        return Err(AppError::new(E_NOT_FOUND, "任务不存在", false));
    };

    Ok(row.get("project_id"))
}

async fn is_job_cancelled(pool: &SqlitePool, job_id: &str) -> bool {
    sqlx::query("SELECT state FROM prompt_jobs WHERE id = ?")
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|row| row.get::<String, _>("state") == "cancelled")
        .unwrap_or(false)
}

async fn set_job_failed(
    pool: &SqlitePool,
    workspace_root: &PathBuf,
    job_id: &str,
    error: &AppError,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let suggestion = suggestion_for_error(error);

    sqlx::query(
        "UPDATE prompt_jobs \
         SET state = 'failed', error_code = ?, error_summary = ?, suggestion = ?, finished_at = ? \
         WHERE id = ?",
    )
    .bind(&error.code)
    .bind(&error.message)
    .bind(suggestion)
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "queue",
        "error",
        &format!("job failed: {}", error.message),
    )
    .await;

    Ok(())
}

async fn set_job_cancelled(
    pool: &SqlitePool,
    workspace_root: &PathBuf,
    job_id: &str,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE prompt_jobs \
         SET state = 'cancelled', error_code = 'E_CANCELLED', error_summary = '任务已取消', suggestion = '如需继续生成，请手动重试任务。', finished_at = ? \
         WHERE id = ?",
    )
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "user_action",
        "info",
        "job marked as cancelled",
    )
    .await;

    Ok(())
}

async fn mark_job_succeeded(
    pool: &SqlitePool,
    workspace_root: &PathBuf,
    job_id: &str,
    project_id: &str,
    artifact_info: artifact::ArtifactInfo,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let artifact_id = format!("artifact_{}", uuid::Uuid::new_v4());

    sqlx::query(
        "INSERT INTO render_artifacts (id, job_id, project_id, file_path, duration_secs, file_size_bytes, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(artifact_id)
    .bind(job_id)
    .bind(project_id)
    .bind(&artifact_info.relative_path)
    .bind(artifact_info.duration_secs)
    .bind(artifact_info.file_size_bytes)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE prompt_jobs \
         SET state = 'succeeded', error_code = NULL, error_summary = NULL, suggestion = NULL, finished_at = ? \
         WHERE id = ?",
    )
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "artifact",
        "info",
        "artifact check finished and job succeeded",
    )
    .await;

    Ok(())
}

fn suggestion_for_error(error: &AppError) -> String {
    if error.code == "E_PROVIDER_RESPONSE_INVALID" {
        return "Provider returned an unexpected response format. Check that the provider type matches the API, the Base URL is the API root only, the endpoint is not duplicated with /chat/completions or /messages, and the provider/proxy returns non-streaming JSON instead of HTML or SSE.".to_string();
    }

    match error.code.as_str() {
        "E_LLM_OUTPUT_INVALID" => {
            "请改写提示词，并明确要求模型只返回一个 Python 代码块。".to_string()
        }
        "E_STATIC_CHECK_FAILED" => {
            "请改写提示词，要求只使用 Manim Community Edition 并避免文件、网络或命令调用。"
                .to_string()
        }
        "E_RENDER_FAIL" => render_failure_suggestion(error),
        E_RENDER_TIMEOUT => "Manim render exceeded 600 seconds (10 minutes) and was terminated. Reduce animation duration, object count, complex 3D work, or LaTeX content, then retry. For longer videos, split the content into shorter scenes.".to_string(),
        "E_ARTIFACT_INVALID" => "渲染已结束，但产物无效。请检查 stderr 日志并重试。".to_string(),
        "E_DEP_MISSING" => {
            "请先修复本地 Python、uv、Manim 或 ffmpeg 依赖，再重新尝试。".to_string()
        }
        _ => "请检查任务日志与 Provider 配置后重试。".to_string(),
    }
}

fn render_failure_suggestion(error: &AppError) -> String {
    let excerpt = render_excerpt_from_error(error).to_ascii_lowercase();
    if excerpt.contains("latex error converting to dvi") {
        return "渲染日志显示 LaTeX 转换失败。请检查 MathTex/Tex 字符串是否为合法 raw LaTeX，并确认 LaTeX/MiKTeX 与 dvisvgm 已安装且在 PATH 中；中文或说明性文字请使用 Text。".to_string();
    }

    if excerpt.contains("config.")
        || excerpt.contains("media_dir")
        || excerpt.contains("output_file")
        || excerpt.contains("video_dir")
    {
        return "渲染日志显示生成代码尝试修改输出路径或 Manim 配置。输出目录由应用统一管理，请重试生成并避免在代码中设置 config、media_dir 或 output_file。".to_string();
    }

    if excerpt.contains("nameerror")
        || excerpt.contains("_animationbuilder")
        || excerpt.contains("parametricsurface")
        || excerpt.contains("cyan")
        || excerpt.contains("magenta")
    {
        return "渲染日志显示生成代码使用了当前 ManimCE 不兼容的 API、颜色常量或动画构造方式。请重试生成；新任务会在静态校验阶段提前拦截此类代码。".to_string();
    }

    if excerpt.contains("typeerror") {
        return "渲染日志显示生成代码向 ManimCE 0.20.1 API 传入了不兼容的参数或对象。请重试生成；新任务会优先使用官方兼容清单中的稳定 API。".to_string();
    }

    "请检查运行环境与渲染日志后重试；若使用 MathTex 或坐标轴公式标签，请确认 LaTeX/MiKTeX 与 dvisvgm 已安装并在 PATH 中。".to_string()
}

fn render_excerpt_from_error(error: &AppError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("renderExcerpt"))
        .and_then(|value| value.as_str())
        .unwrap_or(&error.message)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use uuid::Uuid;

    use super::*;
    use crate::{
        services::{db, workspace},
        types::error_codes::{E_DEP_MISSING, E_RENDER_FAIL, E_RENDER_TIMEOUT},
    };

    #[tokio::test]
    async fn run_queue_worker_processes_jobs_in_fifo_order() {
        let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
        let events = Arc::new(Mutex::new(Vec::new()));

        queue_tx.send("job_a".to_string()).unwrap();
        queue_tx.send("job_b".to_string()).unwrap();
        drop(queue_tx);

        run_queue_worker(queue_rx, |job_id| {
            let events = events.clone();
            async move {
                events.lock().unwrap().push(format!("start:{job_id}"));
                if job_id == "job_a" {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                events.lock().unwrap().push(format!("end:{job_id}"));
            }
        })
        .await;

        assert_eq!(
            events.lock().unwrap().as_slice(),
            ["start:job_a", "end:job_a", "start:job_b", "end:job_b"]
        );
    }

    #[tokio::test]
    async fn claim_job_running_only_transitions_queued_job_once() {
        let (pool, workspace_root, job_id) = setup_job_record("queued").await;

        assert!(claim_job_running(&pool, &job_id).await);

        let row = sqlx::query("SELECT state, started_at FROM prompt_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let state: String = row.get("state");
        let started_at: Option<String> = row.get("started_at");

        assert_eq!(state, "running");
        assert!(started_at.is_some());
        assert!(!claim_job_running(&pool, &job_id).await);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn set_job_failed_persists_suggestion_and_queue_log() {
        let (pool, workspace_root, job_id) = setup_job_record("running").await;
        let error = AppError::new(E_DEP_MISSING, "未检测到 uv，无法启动 Manim 渲染", false);

        set_job_failed(&pool, &workspace_root, &job_id, &error)
            .await
            .unwrap();

        let job_row = sqlx::query(
            "SELECT state, error_code, error_summary, suggestion, finished_at FROM prompt_jobs WHERE id = ?",
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let state: String = job_row.get("state");
        let error_code: Option<String> = job_row.get("error_code");
        let error_summary: Option<String> = job_row.get("error_summary");
        let suggestion: Option<String> = job_row.get("suggestion");
        let finished_at: Option<String> = job_row.get("finished_at");

        assert_eq!(state, "failed");
        assert_eq!(error_code.as_deref(), Some(E_DEP_MISSING));
        assert_eq!(
            error_summary.as_deref(),
            Some("未检测到 uv，无法启动 Manim 渲染")
        );
        assert_eq!(
            suggestion.as_deref(),
            Some("请先修复本地 Python、uv、Manim 或 ffmpeg 依赖，再重新尝试。")
        );
        assert!(finished_at.is_some());

        let log_row = sqlx::query(
            "SELECT stage, level, message FROM job_logs WHERE job_id = ? ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(&job_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let stage: String = log_row.get("stage");
        let level: String = log_row.get("level");
        let message: String = log_row.get("message");

        assert_eq!(stage, "queue");
        assert_eq!(level, "error");
        assert!(message.contains("job failed"));

        cleanup(workspace_root).await;
    }

    #[test]
    fn render_failure_suggestion_classifies_generated_code_compatibility_errors() {
        let cases = [
            "NameError: name 'CYAN' is not defined",
            "NameError: name 'ParametricSurface' is not defined",
            "TypeError: object of type '_AnimationBuilder' has no len()",
        ];

        for excerpt in cases {
            let error = AppError::new(E_RENDER_FAIL, "Manim 渲染失败", false)
                .with_details(serde_json::json!({ "renderExcerpt": excerpt }));

            let suggestion = suggestion_for_error(&error);

            assert!(suggestion.contains("不兼容"));
            assert!(suggestion.contains("静态校验"));
        }
    }

    #[test]
    fn render_failure_suggestion_classifies_latex_errors() {
        let error =
            AppError::new(E_RENDER_FAIL, "Manim 渲染失败", false).with_details(serde_json::json!({
                "renderExcerpt": "ValueError: latex error converting to dvi. See log output above"
            }));

        let suggestion = suggestion_for_error(&error);

        assert!(suggestion.contains("LaTeX"));
        assert!(suggestion.contains("MathTex"));
        assert!(suggestion.contains("dvisvgm"));
    }

    #[test]
    fn render_timeout_suggestion_explains_retry_options() {
        let error = AppError::new(
            E_RENDER_TIMEOUT,
            "Manim render exceeded 600 seconds (10 minutes) and was terminated",
            false,
        );

        let suggestion = suggestion_for_error(&error);

        assert!(suggestion.contains("600 seconds"));
        assert!(suggestion.contains("10 minutes"));
        assert!(suggestion.contains("Reduce animation duration"));
        assert!(suggestion.contains("shorter scenes"));
    }

    #[test]
    fn render_failure_suggestion_classifies_type_errors_and_config_pollution() {
        let type_error = AppError::new(E_RENDER_FAIL, "Manim 渲染失败", false).with_details(
            serde_json::json!({
                "renderExcerpt": "TypeError: Mobject.set_style() got an unexpected keyword argument 'dash_length'"
            }),
        );
        let config_error =
            AppError::new(E_RENDER_FAIL, "Manim 渲染失败", false).with_details(serde_json::json!({
                "renderExcerpt": "RuntimeError: config.media_dir cannot be changed during render"
            }));

        let type_suggestion = suggestion_for_error(&type_error);
        let config_suggestion = suggestion_for_error(&config_error);

        assert!(type_suggestion.contains("ManimCE 0.20.1"));
        assert!(type_suggestion.contains("不兼容的参数"));
        assert!(config_suggestion.contains("输出目录由应用统一管理"));
        assert!(config_suggestion.contains("config"));
    }

    async fn setup_job_record(state: &str) -> (SqlitePool, PathBuf, String) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-queue-tests-{}", Uuid::new_v4()));
        workspace::create_standard_dirs(&workspace_root)
            .await
            .unwrap();

        let pool = db::open_or_create(&workspace_root.join("db").join("app.sqlite"))
            .await
            .unwrap();
        let now = Utc::now().to_rfc3339();
        let provider_id = format!("provider_{}", Uuid::new_v4());
        let project_id = format!("project_{}", Uuid::new_v4());
        let job_id = format!("job_{}", Uuid::new_v4());

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
            "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, ?, NULL, NULL)",
        )
        .bind(&job_id)
        .bind(&project_id)
        .bind(&provider_id)
        .bind("Explain quadratic formula")
        .bind(state)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        (pool, workspace_root, job_id)
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
