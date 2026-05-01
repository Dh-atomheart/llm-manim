use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::Row;
use tauri::{AppHandle, Manager, State};
use tokio::process::Command;

use crate::{
    services::{db, workspace as workspace_service},
    state::AppState,
    types::{
        error_codes::{E_DB, E_IO, E_RUNTIME_INVALID, E_WORKSPACE_INVALID},
        response::{AppError, AppResponse},
    },
};

const WORKSPACE_CONFIG_ID: &str = "default";
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatus {
    configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_path: Option<String>,
    writable: bool,
    database_ready: bool,
    runtime_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInitResult {
    workspace_path: String,
    created: bool,
    database_ready: bool,
}

#[derive(Debug, Serialize)]
pub struct RuntimeComponentInfo {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    status: String,
    python: RuntimeComponentInfo,
    uv: RuntimeComponentInfo,
    manim: RuntimeComponentInfo,
    uv_manim: RuntimeComponentInfo,
    ffmpeg: RuntimeComponentInfo,
    ffprobe: RuntimeComponentInfo,
    latex: RuntimeComponentInfo,
    dvisvgm: RuntimeComponentInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    message: String,
}

#[tauri::command]
pub fn get_workspace_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResponse<WorkspaceStatus> {
    tauri::async_runtime::block_on(get_workspace_status_inner(app, state.inner()))
}

async fn get_workspace_status_inner(
    app: AppHandle,
    state: &AppState,
) -> AppResponse<WorkspaceStatus> {
    let workspace_path = match current_workspace_path(&state, &app).await {
        Ok(path) => path,
        Err(error) => return AppResponse::err(error),
    };
    let workspace_root = workspace_path.as_deref().map(PathBuf::from);
    let runtime = collect_runtime_status(workspace_root.as_deref()).await;

    let Some(workspace_path) = workspace_path else {
        return AppResponse::ok(WorkspaceStatus {
            configured: false,
            workspace_path: None,
            writable: false,
            database_ready: false,
            runtime_status: runtime.status,
        });
    };

    let workspace_root = workspace_root.expect("workspace path was checked above");
    let writable = workspace_service::check_writable(&workspace_root).await;
    let database_ready = workspace_database_ready(&state, &workspace_root).await;

    AppResponse::ok(WorkspaceStatus {
        configured: true,
        workspace_path: Some(workspace_path),
        writable,
        database_ready,
        runtime_status: runtime.status,
    })
}

#[tauri::command]
pub fn initialize_workspace(
    workspace_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResponse<WorkspaceInitResult> {
    tauri::async_runtime::block_on(initialize_workspace_inner(
        workspace_path,
        app,
        state.inner(),
    ))
}

async fn initialize_workspace_inner(
    workspace_path: String,
    app: AppHandle,
    state: &AppState,
) -> AppResponse<WorkspaceInitResult> {
    let trimmed = workspace_path.trim();
    if trimmed.is_empty() {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区路径不能为空",
            false,
        ));
    }

    let workspace_root = PathBuf::from(trimmed);
    if workspace_root.exists() && !workspace_root.is_dir() {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区路径必须是目录",
            false,
        ));
    }

    let created = !workspace_root.exists();
    if let Err(error) = workspace_service::create_standard_dirs(&workspace_root).await {
        return AppResponse::err(io_error(E_IO, format!("无法创建工作区目录: {error}")));
    }

    if !workspace_service::check_writable(&workspace_root).await {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区不可写，请选择其他目录",
            false,
        ));
    }

    let database_path = workspace_root.join("db").join("app.sqlite");
    let pool = match db::open_or_create(&database_path).await {
        Ok(pool) => pool,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法初始化 SQLite 数据库: {error}"),
                false,
            ));
        }
    };

    if let Err(error) = upsert_workspace_config(&pool, trimmed).await {
        return AppResponse::err(AppError::new(
            E_DB,
            format!("无法写入工作区配置: {error}"),
            false,
        ));
    }

    if let Err(error) = workspace_service::write_workspace_json(&workspace_root).await {
        return AppResponse::err(io_error(E_IO, format!("无法写入 workspace.json: {error}")));
    }

    let app_data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(error) => {
            return AppResponse::err(io_error(E_IO, format!("无法定位应用数据目录: {error}")));
        }
    };

    if let Err(error) = workspace_service::save_app_settings(&app_data_dir, trimmed).await {
        return AppResponse::err(io_error(E_IO, format!("无法保存工作区配置: {error}")));
    }

    state.set_workspace(trimmed.to_string(), pool).await;

    AppResponse::ok(WorkspaceInitResult {
        workspace_path: trimmed.to_string(),
        created,
        database_ready: true,
    })
}

#[tauri::command]
pub fn check_runtime(
    workspace_path: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResponse<RuntimeStatus> {
    tauri::async_runtime::block_on(async {
        let explicit_workspace = workspace_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from);
        let workspace_root = match explicit_workspace {
            Some(path) => Some(path),
            None => match current_workspace_path(state.inner(), &app).await {
                Ok(path) => path.map(PathBuf::from),
                Err(error) => return AppResponse::err(error),
            },
        };

        AppResponse::ok(collect_runtime_status(workspace_root.as_deref()).await)
    })
}

pub async fn restore_workspace_state(app: &AppHandle, state: &AppState) {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("failed to resolve app data dir: {error}");
            return;
        }
    };

    let Some(workspace_path) = (match workspace_service::read_app_settings(&app_data_dir).await {
        Ok(path) => path,
        Err(error) => {
            eprintln!("failed to read app settings: {error}");
            return;
        }
    }) else {
        return;
    };

    let database_path = Path::new(&workspace_path).join("db").join("app.sqlite");
    match db::open_or_create(&database_path).await {
        Ok(pool) => {
            state.set_workspace(workspace_path.clone(), pool).await;

            if let Err(error) = workspace_service::clean_temp_dir(Path::new(&workspace_path)).await
            {
                eprintln!("failed to clean workspace temp dir: {error}");
            }
        }
        Err(error) => eprintln!("failed to open workspace database: {error}"),
    }
}

async fn current_workspace_path(
    state: &AppState,
    app: &AppHandle,
) -> Result<Option<String>, AppError> {
    if let Some(workspace_path) = state.get_workspace_path().await {
        return Ok(Some(workspace_path));
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| io_error(E_IO, format!("无法定位应用数据目录: {error}")))?;

    workspace_service::read_app_settings(&app_data_dir)
        .await
        .map_err(|error| io_error(E_IO, format!("无法读取工作区配置: {error}")))
}

async fn workspace_database_ready(state: &AppState, workspace_root: &Path) -> bool {
    if let Some(pool) = state.get_db().await {
        return sqlx::query("SELECT 1").fetch_one(&pool).await.is_ok();
    }

    let database_path = workspace_root.join("db").join("app.sqlite");
    if !tokio::fs::try_exists(&database_path).await.unwrap_or(false) {
        return false;
    }

    match db::open_existing(&database_path).await {
        Ok(pool) => sqlx::query("SELECT 1").fetch_one(&pool).await.is_ok(),
        Err(_) => false,
    }
}

async fn upsert_workspace_config(
    pool: &sqlx::SqlitePool,
    workspace_path: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO workspace_config (id, workspace_path, schema_version, created_at, updated_at) VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET workspace_path = excluded.workspace_path, schema_version = excluded.schema_version, updated_at = excluded.updated_at",
    )
    .bind(WORKSPACE_CONFIG_ID)
    .bind(workspace_path)
    .bind(SCHEMA_VERSION)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let _ = sqlx::query("SELECT id FROM workspace_config WHERE id = ?")
        .bind(WORKSPACE_CONFIG_ID)
        .fetch_one(pool)
        .await?
        .get::<String, _>("id");

    Ok(())
}

async fn collect_runtime_status(workspace_root: Option<&Path>) -> RuntimeStatus {
    let python = probe_command("python", &["--version"]).await;
    let uv = probe_command("uv", &["--version"]).await;
    let manim = probe_command("manim", &["--version"]).await;
    let uv_manim = probe_command_in_dir(
        "uv",
        &["run", "--with", "manim", "manim", "--version"],
        workspace_root,
    )
    .await;
    let ffmpeg = probe_command("ffmpeg", &["-version"]).await;
    let ffprobe = probe_command("ffprobe", &["-version"]).await;
    let latex = probe_command("latex", &["--version"]).await;
    let dvisvgm = probe_command("dvisvgm", &["--version"]).await;

    let components = [&python, &uv, &uv_manim, &ffmpeg, &ffprobe, &latex, &dvisvgm];
    let (status, error_code, message) = runtime_status_summary(&components);

    RuntimeStatus {
        status: status.to_string(),
        python,
        uv,
        manim,
        uv_manim,
        ffmpeg,
        ffprobe,
        latex,
        dvisvgm,
        error_code,
        message,
    }
}

async fn probe_command(program: &str, args: &[&str]) -> RuntimeComponentInfo {
    probe_command_in_dir(program, args, None).await
}

async fn probe_command_in_dir(
    program: &str,
    args: &[&str],
    current_dir: Option<&Path>,
) -> RuntimeComponentInfo {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }

    match command.output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let text = if stdout.trim().is_empty() {
                &stderr
            } else {
                &stdout
            };
            let version = extract_version(text);
            RuntimeComponentInfo {
                status: "ok".to_string(),
                version,
            }
        }
        _ => RuntimeComponentInfo {
            status: "missing".to_string(),
            version: None,
        },
    }
}

fn extract_version(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let cleaned = word.trim_start_matches('v');
        let cleaned = cleaned.trim_end_matches(|c: char| c == ',' || c == ';');
        let first = cleaned.chars().next();
        if first.map(|c| c.is_ascii_digit()).unwrap_or(false) && cleaned.contains('.') {
            return Some(cleaned.to_string());
        }
    }
    None
}

fn io_error(code: &str, message: String) -> AppError {
    AppError::new(code, message, false)
}

fn runtime_status_summary(
    components: &[&RuntimeComponentInfo],
) -> (&'static str, Option<String>, String) {
    let all_ready = components.iter().all(|component| component.status == "ok");
    let any_ready = components.iter().any(|component| component.status == "ok");

    if all_ready {
        (
            "ready",
            None,
            "运行环境可用，可在当前工作区启动 Manim 渲染，公式渲染所需 LaTeX/dvisvgm 已就绪"
                .to_string(),
        )
    } else if any_ready {
        (
            "broken",
            Some(E_RUNTIME_INVALID.to_string()),
            "部分运行依赖缺失，无法在当前工作区启动 Manim 渲染；如需 MathTex 或坐标轴公式标签，请安装 MiKTeX/LaTeX 并确保 dvisvgm 在 PATH 中".to_string(),
        )
    } else {
        (
            "missing",
            Some(E_RUNTIME_INVALID.to_string()),
            "未检测到所需运行依赖".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn component(status: &str) -> RuntimeComponentInfo {
        RuntimeComponentInfo {
            status: status.to_string(),
            version: None,
        }
    }

    #[test]
    fn runtime_status_summary_requires_uv_managed_manim() {
        let python = component("ok");
        let uv = component("ok");
        let uv_manim = component("missing");
        let ffmpeg = component("ok");
        let ffprobe = component("ok");
        let latex = component("ok");
        let dvisvgm = component("ok");

        let (status, error_code, message) =
            runtime_status_summary(&[&python, &uv, &uv_manim, &ffmpeg, &ffprobe, &latex, &dvisvgm]);

        assert_eq!(status, "broken");
        assert_eq!(error_code.as_deref(), Some(E_RUNTIME_INVALID));
        assert!(message.contains("Manim 渲染"));
    }

    #[test]
    fn runtime_status_summary_is_ready_without_global_manim() {
        let python = component("ok");
        let uv = component("ok");
        let uv_manim = component("ok");
        let ffmpeg = component("ok");
        let ffprobe = component("ok");
        let latex = component("ok");
        let dvisvgm = component("ok");

        let (status, error_code, _) =
            runtime_status_summary(&[&python, &uv, &uv_manim, &ffmpeg, &ffprobe, &latex, &dvisvgm]);

        assert_eq!(status, "ready");
        assert!(error_code.is_none());
    }

    #[test]
    fn runtime_status_summary_requires_latex_and_dvisvgm_for_formula_rendering() {
        let python = component("ok");
        let uv = component("ok");
        let uv_manim = component("ok");
        let ffmpeg = component("ok");
        let ffprobe = component("ok");
        let latex = component("missing");
        let dvisvgm = component("missing");

        let (status, error_code, message) =
            runtime_status_summary(&[&python, &uv, &uv_manim, &ffmpeg, &ffprobe, &latex, &dvisvgm]);

        assert_eq!(status, "broken");
        assert_eq!(error_code.as_deref(), Some(E_RUNTIME_INVALID));
        assert!(message.contains("LaTeX"));
        assert!(message.contains("dvisvgm"));
    }

    #[test]
    fn workspace_status_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(WorkspaceStatus {
            configured: true,
            workspace_path: Some("F:/workspace".to_string()),
            writable: true,
            database_ready: true,
            runtime_status: "ready".to_string(),
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "configured": true,
                "workspacePath": "F:/workspace",
                "writable": true,
                "databaseReady": true,
                "runtimeStatus": "ready"
            })
        );
    }

    #[test]
    fn workspace_init_result_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(WorkspaceInitResult {
            workspace_path: "F:/workspace".to_string(),
            created: false,
            database_ready: true,
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "workspacePath": "F:/workspace",
                "created": false,
                "databaseReady": true
            })
        );
    }
}
