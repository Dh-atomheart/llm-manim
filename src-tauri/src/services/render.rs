use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde_json::json;
use sqlx::SqlitePool;
use tokio::{
    fs,
    process::Command,
    time::{sleep, Duration, Instant},
};

use crate::{
    services::{logging, redact},
    types::{
        error_codes::{
            E_CANCELLED, E_CANCEL_FAILED, E_DEP_MISSING, E_IO, E_RENDER_FAIL, E_RENDER_TIMEOUT,
        },
        response::AppError,
    },
};

const RENDER_TIMEOUT_STAGE_SECS: &[u64] = &[120, 180, 300];

const MANIM_CFG: &str = concat!(
    "[CLI]\n",
    "quality = medium_quality\n",
    "format = mp4\n",
    "frame_rate = 30\n",
    "pixel_height = 720\n",
    "pixel_width = 1280\n",
);

pub async fn write_scene_file(
    workspace_root: &Path,
    job_id: &str,
    code: &str,
) -> Result<PathBuf, AppError> {
    let job_dir = workspace_root.join("jobs").join(job_id);
    fs::create_dir_all(&job_dir).await.map_err(io_error)?;

    let scene_path = job_dir.join("generated_scene.py");
    fs::write(&scene_path, code).await.map_err(io_error)?;

    Ok(scene_path)
}

pub async fn write_manim_cfg(workspace_root: &Path, job_id: &str) -> Result<PathBuf, AppError> {
    let job_dir = workspace_root.join("jobs").join(job_id);
    fs::create_dir_all(&job_dir).await.map_err(io_error)?;

    let cfg_path = job_dir.join("manim.cfg");
    fs::write(&cfg_path, MANIM_CFG).await.map_err(io_error)?;

    Ok(cfg_path)
}

pub async fn run_manim(
    pool: Option<&SqlitePool>,
    workspace_root: &Path,
    job_id: &str,
    project_id: &str,
    scene_name: &str,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), AppError> {
    run_manim_with_command_builder(
        pool,
        workspace_root,
        job_id,
        project_id,
        scene_name,
        cancel_flag,
        build_manim_command,
    )
    .await
}

async fn run_manim_with_command_builder<F>(
    pool: Option<&SqlitePool>,
    workspace_root: &Path,
    job_id: &str,
    project_id: &str,
    scene_name: &str,
    cancel_flag: Arc<AtomicBool>,
    command_builder: F,
) -> Result<(), AppError>
where
    F: Fn(&Path, &str, &str, &str) -> Command,
{
    let timeout_stages = render_timeout_stages();
    run_manim_with_command_builder_and_timeout(
        pool,
        workspace_root,
        job_id,
        project_id,
        scene_name,
        cancel_flag,
        &timeout_stages,
        command_builder,
    )
    .await
}

async fn run_manim_with_command_builder_and_timeout<F>(
    pool: Option<&SqlitePool>,
    workspace_root: &Path,
    job_id: &str,
    project_id: &str,
    scene_name: &str,
    cancel_flag: Arc<AtomicBool>,
    render_timeout_stages: &[Duration],
    command_builder: F,
) -> Result<(), AppError>
where
    F: Fn(&Path, &str, &str, &str) -> Command,
{
    let render_timeout_stages = normalize_timeout_stages(render_timeout_stages);
    let timeout_stage_secs: Vec<u64> = render_timeout_stages
        .iter()
        .map(Duration::as_secs)
        .collect();
    let total_timeout = render_timeout_stages
        .iter()
        .copied()
        .fold(Duration::ZERO, |total, stage| total + stage);

    let job_dir = workspace_root.join("jobs").join(job_id);
    fs::create_dir_all(&job_dir).await.map_err(io_error)?;

    let stdout_path = job_dir.join("render_stdout.log");
    let stderr_path = job_dir.join("render_stderr.log");
    let stdout_file = File::create(&stdout_path).map_err(io_error_sync)?;
    let stderr_file = File::create(&stderr_path).map_err(io_error_sync)?;

    let media_dir = PathBuf::from("artifacts")
        .join(project_id)
        .join(job_id)
        .join("media");
    fs::create_dir_all(workspace_root.join(&media_dir))
        .await
        .map_err(io_error)?;

    logging::write_job_log(
        pool,
        Some(workspace_root),
        job_id,
        "render",
        "info",
        "render process started",
    )
    .await;

    let mut command = command_builder(workspace_root, job_id, project_id, scene_name);
    let mut child = command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(map_spawn_error)?;
    let started_at = Instant::now();
    let mut current_stage = 0usize;
    let mut current_deadline = render_timeout_stages[current_stage];

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            let kill_result = child.start_kill();
            let _ = child.wait().await;

            return match kill_result {
                Ok(()) => {
                    logging::write_job_log(
                        pool,
                        Some(workspace_root),
                        job_id,
                        "render",
                        "warn",
                        "render process cancelled",
                    )
                    .await;
                    Err(AppError::new(E_CANCELLED, "任务已取消", false))
                }
                Err(error) => Err(AppError::new(
                    E_CANCEL_FAILED,
                    format!("取消渲染进程失败: {error}"),
                    false,
                )),
            };
        }

        while started_at.elapsed() >= current_deadline {
            if current_stage + 1 < render_timeout_stages.len() {
                let next_stage = render_timeout_stages[current_stage + 1];
                logging::write_job_log(
                    pool,
                    Some(workspace_root),
                    job_id,
                    "render",
                    "warn",
                    &format!(
                        "render process still running after {} seconds; extending timeout by {} seconds",
                        current_deadline.as_secs(),
                        next_stage.as_secs()
                    ),
                )
                .await;
                current_stage += 1;
                current_deadline += render_timeout_stages[current_stage];
                continue;
            }

            let kill_result = child.start_kill();
            let _ = child.wait().await;

            let mut details = json!({
                "timeoutSecs": total_timeout.as_secs(),
                "timeoutStagesSecs": timeout_stage_secs,
            });
            if let Err(error) = kill_result {
                details = json!({
                    "timeoutSecs": total_timeout.as_secs(),
                    "timeoutStagesSecs": timeout_stage_secs,
                    "killError": error.to_string(),
                });
            }

            logging::write_job_log(
                pool,
                Some(workspace_root),
                job_id,
                "render",
                "warn",
                &format!(
                    "render process timed out after {} seconds",
                    total_timeout.as_secs()
                ),
            )
            .await;

            return Err(AppError::new(
                E_RENDER_TIMEOUT,
                format!(
                    "Manim render exceeded {} seconds (10 minutes) and was terminated",
                    total_timeout.as_secs()
                ),
                false,
            )
            .with_details(details));
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    logging::write_job_log(
                        pool,
                        Some(workspace_root),
                        job_id,
                        "render",
                        "info",
                        "render process exited successfully",
                    )
                    .await;
                    return Ok(());
                }

                let render_excerpt = summarize_render_logs(&stderr_path, &stdout_path).await;
                logging::write_job_log(
                    pool,
                    Some(workspace_root),
                    job_id,
                    "render",
                    "error",
                    &format!(
                        "render process exited with failure: {}",
                        render_excerpt
                            .as_deref()
                            .unwrap_or("no render output summary available")
                    ),
                )
                .await;
                let mut error = AppError::new(E_RENDER_FAIL, "Manim 渲染失败", false);
                if let Some(render_excerpt) = render_excerpt {
                    error = error.with_details(json!({ "renderExcerpt": render_excerpt }));
                }
                return Err(error);
            }
            Ok(None) => sleep(Duration::from_millis(200)).await,
            Err(error) => {
                return Err(AppError::new(
                    E_RENDER_FAIL,
                    format!("无法等待渲染进程退出: {error}"),
                    false,
                ))
            }
        }
    }
}

fn render_timeout_stages() -> Vec<Duration> {
    RENDER_TIMEOUT_STAGE_SECS
        .iter()
        .copied()
        .map(Duration::from_secs)
        .collect()
}

fn normalize_timeout_stages(stages: &[Duration]) -> Vec<Duration> {
    let normalized = stages
        .iter()
        .copied()
        .filter(|stage| !stage.is_zero())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        render_timeout_stages()
    } else {
        normalized
    }
}

fn build_manim_command(
    workspace_root: &Path,
    job_id: &str,
    project_id: &str,
    scene_name: &str,
) -> Command {
    let scene_path = PathBuf::from("jobs")
        .join(job_id)
        .join("generated_scene.py");
    let cfg_path = PathBuf::from("jobs").join(job_id).join("manim.cfg");
    let media_dir = PathBuf::from("artifacts")
        .join(project_id)
        .join(job_id)
        .join("media");

    let mut command = Command::new("uv");
    command
        .arg("run")
        .arg("--with")
        .arg("manim")
        .arg("manim")
        .arg("--config_file")
        .arg(&cfg_path)
        .arg(&scene_path)
        .arg(scene_name)
        .arg("-qm")
        .arg("--format=mp4")
        .arg("--media_dir")
        .arg(&media_dir)
        .current_dir(workspace_root);
    command
}

async fn summarize_render_logs(stderr_path: &Path, stdout_path: &Path) -> Option<String> {
    let stderr_summary = summarize_log(stderr_path).await;
    if stderr_summary.is_some() {
        return stderr_summary;
    }

    summarize_log(stdout_path).await
}

async fn summarize_log(path: &Path) -> Option<String> {
    let bytes = fs::read(path).await.ok()?;
    if bytes.is_empty() {
        return None;
    }

    let contents = String::from_utf8_lossy(&bytes);
    let sanitized = redact::redact(contents.as_ref(), &[]);
    let collapsed = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }

    Some(redact::truncate(&focused_log_summary(&collapsed), 280))
}

fn focused_log_summary(collapsed: &str) -> String {
    let error_markers = [
        "TypeError:",
        "ValueError:",
        "AttributeError:",
        "NameError:",
        "ImportError:",
        "ModuleNotFoundError:",
        "RuntimeError:",
        "Exception:",
    ];

    for marker in error_markers {
        if let Some(index) = collapsed.rfind(marker) {
            return collapsed[index..].to_string();
        }
    }

    if let Some(index) = collapsed.rfind("Traceback (most recent call last)") {
        return collapsed[index..].to_string();
    }

    collapsed.to_string()
}

fn io_error(error: std::io::Error) -> AppError {
    AppError::new(E_IO, format!("无法写入渲染文件: {error}"), false)
}

fn io_error_sync(error: std::io::Error) -> AppError {
    AppError::new(E_IO, format!("无法创建渲染日志文件: {error}"), false)
}

fn map_spawn_error(error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return AppError::new(E_DEP_MISSING, "未检测到 uv，无法启动 Manim 渲染", false);
    }

    AppError::new(
        E_RENDER_FAIL,
        format!("无法启动 Manim 渲染进程: {error}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use chrono::Utc;
    use sqlx::Row;
    use uuid::Uuid;

    use super::*;
    use crate::services::{db, workspace};

    #[tokio::test]
    async fn run_manim_reports_successful_exit() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;

        let result = run_manim_with_command_builder(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _| success_command(),
        )
        .await;

        assert!(result.is_ok());

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message == "render process started"));
        assert!(log_messages
            .iter()
            .any(|message| message == "render process exited successfully"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_manim_reports_failure_exit_and_stderr_summary() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;

        let error = run_manim_with_command_builder(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _| failure_command(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_RENDER_FAIL);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("render process exited with failure")));
        assert!(log_messages
            .iter()
            .any(|message| message.contains("render failure")));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn summarize_log_decodes_non_utf8_output_lossily() {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-render-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace_root).await.unwrap();
        let log_path = workspace_root.join("render_stderr.log");
        fs::write(
            &log_path,
            [b'F', b'a', b'i', b'l', 0xFF, b'l', b'a', b't', b'e', b'x'],
        )
        .await
        .unwrap();

        let summary = summarize_log(&log_path).await.unwrap();

        assert!(summary.contains("Fail"));
        assert!(summary.contains("latex"));

        cleanup(workspace_root).await;
    }

    #[test]
    fn focused_log_summary_prefers_final_exception_over_warning_prefix() {
        let summary = focused_log_summary(
            "latex: major issue: update warning + Traceback (most recent call last) frame TypeError: VMobject.set_style() got an unexpected keyword argument 'dash_length'",
        );

        assert!(summary.starts_with("TypeError:"));
        assert!(summary.contains("dash_length"));
        assert!(!summary.starts_with("latex: major issue"));
    }

    #[tokio::test]
    async fn run_manim_falls_back_to_stdout_when_stderr_is_empty() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;

        let error = run_manim_with_command_builder(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _| failure_stdout_command(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_RENDER_FAIL);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("stdout render failure")));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_manim_returns_cancelled_when_process_is_killed() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let signal = cancel_flag.clone();

        tokio::spawn(async move {
            sleep(Duration::from_millis(250)).await;
            signal.store(true, Ordering::Relaxed);
        });

        let error = run_manim_with_command_builder(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            cancel_flag,
            |_, _, _, _| long_running_command(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_CANCELLED);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message == "render process cancelled"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_manim_times_out_and_kills_process() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;

        let error = run_manim_with_command_builder_and_timeout(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            Arc::new(AtomicBool::new(false)),
            &[
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(1),
            ],
            |_, _, _, _| long_running_command(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_RENDER_TIMEOUT);
        let details = error.details.as_ref().expect("timeout details");
        assert_eq!(details.get("timeoutSecs").and_then(|value| value.as_u64()), Some(3));
        assert_eq!(
            details.get("timeoutStagesSecs").and_then(|value| value.as_array()).map(Vec::len),
            Some(3)
        );

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("render process still running after 1 seconds; extending timeout by 1 seconds")));
        assert!(log_messages
            .iter()
            .any(|message| message.contains("render process still running after 2 seconds; extending timeout by 1 seconds")));
        assert_eq!(
            log_messages
                .iter()
                .filter(|message| message.contains("render process still running after"))
                .count(),
            2
        );
        assert!(log_messages
            .iter()
            .any(|message| message.contains("render process timed out after 3 seconds")));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_manim_returns_dep_missing_when_command_is_unavailable() {
        let (pool, workspace_root, job_id, project_id) = setup_render_context().await;

        let error = run_manim_with_command_builder(
            Some(&pool),
            &workspace_root,
            &job_id,
            &project_id,
            "Demo",
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _| missing_command(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_DEP_MISSING);

        cleanup(workspace_root).await;
    }

    async fn setup_render_context() -> (SqlitePool, PathBuf, String, String) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-render-tests-{}", Uuid::new_v4()));
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
        .bind("Render demo scene")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        (pool, workspace_root, job_id, project_id)
    }

    async fn load_job_log_messages(pool: &SqlitePool, job_id: &str) -> Vec<String> {
        sqlx::query("SELECT message FROM job_logs WHERE job_id = ? ORDER BY timestamp ASC")
            .bind(job_id)
            .fetch_all(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("message"))
            .collect()
    }

    #[cfg(target_os = "windows")]
    fn success_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "exit 0"]);
        command
    }

    #[cfg(not(target_os = "windows"))]
    fn success_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        command
    }

    #[cfg(target_os = "windows")]
    fn failure_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "echo render failure 1>&2 & exit 1"]);
        command
    }

    #[cfg(not(target_os = "windows"))]
    fn failure_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "echo render failure 1>&2; exit 1"]);
        command
    }

    #[cfg(target_os = "windows")]
    fn failure_stdout_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "echo stdout render failure & exit 1"]);
        command
    }

    #[cfg(not(target_os = "windows"))]
    fn failure_stdout_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "echo stdout render failure; exit 1"]);
        command
    }

    #[cfg(target_os = "windows")]
    fn long_running_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping -n 6 127.0.0.1 >nul"]);
        command
    }

    #[cfg(not(target_os = "windows"))]
    fn long_running_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        command
    }

    fn missing_command() -> Command {
        Command::new("__manim4learn_missing_command__")
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = fs::remove_dir_all(workspace_root).await;
    }
}
