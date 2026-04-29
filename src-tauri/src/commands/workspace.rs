use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::Row;
use tauri::{AppHandle, Manager, State};
use tokio::process::Command;

use crate::{
    services::{db, workspace as workspace_service},
    state::AppState,
    types::{
        error_codes::{E_DB, E_IO, E_WORKSPACE_INVALID},
        response::{AppError, AppResponse},
    },
};

const WORKSPACE_CONFIG_ID: &str = "default";
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Serialize)]
pub struct WorkspaceStatus {
    configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_path: Option<String>,
    writable: bool,
    database_ready: bool,
    runtime_status: String,
}

#[derive(Debug, Serialize)]
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
pub struct RuntimeStatus {
    status: String,
    python: RuntimeComponentInfo,
    uv: RuntimeComponentInfo,
    manim: RuntimeComponentInfo,
    ffmpeg: RuntimeComponentInfo,
    message: String,
}

#[tauri::command]
pub fn get_workspace_status(app: AppHandle, state: State<'_, AppState>) -> AppResponse<WorkspaceStatus> {
    tauri::async_runtime::block_on(get_workspace_status_inner(app, state.inner()))
}

async fn get_workspace_status_inner(
    app: AppHandle,
    state: &AppState,
) -> AppResponse<WorkspaceStatus> {
    let runtime = collect_runtime_status().await;

    let workspace_path = match current_workspace_path(&state, &app).await {
        Ok(path) => path,
        Err(error) => return AppResponse::err(error),
    };

    let Some(workspace_path) = workspace_path else {
        return AppResponse::ok(WorkspaceStatus {
            configured: false,
            workspace_path: None,
            writable: false,
            database_ready: false,
            runtime_status: runtime.status,
        });
    };

    let workspace_root = PathBuf::from(&workspace_path);
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
        return AppResponse::err(io_error(
            E_IO,
            format!("无法创建工作区目录: {error}"),
        ));
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
        return AppResponse::err(io_error(
            E_IO,
            format!("无法写入 workspace.json: {error}"),
        ));
    }

    let app_data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(error) => {
            return AppResponse::err(io_error(
                E_IO,
                format!("无法定位应用数据目录: {error}"),
            ));
        }
    };

    if let Err(error) = workspace_service::save_app_settings(&app_data_dir, trimmed).await {
        return AppResponse::err(io_error(
            E_IO,
            format!("无法保存工作区配置: {error}"),
        ));
    }

    state.set_workspace(trimmed.to_string(), pool).await;

    AppResponse::ok(WorkspaceInitResult {
        workspace_path: trimmed.to_string(),
        created,
        database_ready: true,
    })
}

#[tauri::command]
pub fn check_runtime() -> AppResponse<RuntimeStatus> {
    tauri::async_runtime::block_on(async { AppResponse::ok(collect_runtime_status().await) })
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
        Ok(pool) => state.set_workspace(workspace_path, pool).await,
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

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        io_error(
            E_IO,
            format!("无法定位应用数据目录: {error}"),
        )
    })?;

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

async fn upsert_workspace_config(pool: &sqlx::SqlitePool, workspace_path: &str) -> Result<(), sqlx::Error> {
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

async fn collect_runtime_status() -> RuntimeStatus {
    let python = probe_command("python", &["--version"]).await;
    let uv = probe_command("uv", &["--version"]).await;
    let manim = probe_command("manim", &["--version"]).await;
    let ffmpeg = probe_command("ffmpeg", &["-version"]).await;

    let all_ready = [&python, &uv, &manim, &ffmpeg].iter().all(|c| c.status == "ok");
    let any_ready = [&python, &uv, &manim, &ffmpeg].iter().any(|c| c.status == "ok");

    let (status, message) = if all_ready {
        ("ready", "运行环境可用")
    } else if any_ready {
        ("broken", "部分运行依赖缺失")
    } else {
        ("missing", "未检测到所需运行依赖")
    };

    RuntimeStatus {
        status: status.to_string(),
        python,
        uv,
        manim,
        ffmpeg,
        message: message.to_string(),
    }
}

async fn probe_command(program: &str, args: &[&str]) -> RuntimeComponentInfo {
    match Command::new(program).args(args).output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let text = if stdout.trim().is_empty() { &stderr } else { &stdout };
            let version = extract_version(text);
            RuntimeComponentInfo { status: "ok".to_string(), version }
        }
        _ => RuntimeComponentInfo { status: "missing".to_string(), version: None },
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