use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use tauri::State;
use tokio::fs;
use uuid::Uuid;

use crate::{
    state::AppState,
    types::{
        error_codes::{E_DB, E_IO, E_NOT_FOUND, E_VALIDATION, E_WORKSPACE_INVALID},
        response::{AppError, AppResponse},
    },
};

#[derive(Debug, Serialize)]
pub struct Project {
    id: String,
    name: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct EmptyData {}

#[tauri::command]
pub fn create_project(name: String, state: State<'_, AppState>) -> AppResponse<Project> {
    tauri::async_runtime::block_on(create_project_inner(name, state.inner()))
}

async fn create_project_inner(name: String, state: &AppState) -> AppResponse<Project> {
    let project_name = name.trim();
    if project_name.is_empty() {
        return AppResponse::err(AppError::new(E_VALIDATION, "项目名称不能为空", false));
    }

    if project_name.chars().count() > 100 {
        return AppResponse::err(AppError::new(
            E_VALIDATION,
            "项目名称不能超过 100 个字符",
            false,
        ));
    }

    let Some(pool) = state.get_db().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };
    let Some(workspace_path) = state.get_workspace_path().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };

    let project_id = format!("project_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();

    if let Err(error) = sqlx::query(
        "INSERT INTO projects (id, name, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, NULL)",
    )
    .bind(&project_id)
    .bind(project_name)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    {
        return AppResponse::err(AppError::new(
            E_DB,
            format!("无法创建项目记录: {error}"),
            false,
        ));
    }

    let project_dir = PathBuf::from(workspace_path).join("projects").join(&project_id);
    if let Err(error) = fs::create_dir_all(&project_dir).await {
        let _ = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(&project_id)
            .execute(&pool)
            .await;

        return AppResponse::err(AppError::new(
            E_IO,
            format!("无法创建项目目录: {error}"),
            false,
        ));
    }

    AppResponse::ok(Project {
        id: project_id,
        name: project_name.to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> AppResponse<Vec<Project>> {
    tauri::async_runtime::block_on(list_projects_inner(state.inner()))
}

async fn list_projects_inner(state: &AppState) -> AppResponse<Vec<Project>> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::ok(Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT id, name, created_at, updated_at FROM projects WHERE deleted_at IS NULL ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取项目列表: {error}"),
                false,
            ));
        }
    };

    let projects = rows
        .into_iter()
        .map(|row| Project {
            id: row.get("id"),
            name: row.get("name"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    AppResponse::ok(projects)
}

#[tauri::command]
pub fn delete_project(id: String, state: State<'_, AppState>) -> AppResponse<EmptyData> {
    tauri::async_runtime::block_on(delete_project_inner(id, state.inner()))
}

async fn delete_project_inner(id: String, state: &AppState) -> AppResponse<EmptyData> {
    if !id.starts_with("project_") {
        return AppResponse::err(AppError::new(E_VALIDATION, "非法项目 ID", false));
    }

    let Some(pool) = state.get_db().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };
    let Some(workspace_path) = state.get_workspace_path().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };

    let now = Utc::now().to_rfc3339();
    let result = match sqlx::query(
        "UPDATE projects SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&pool)
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法删除项目: {error}"),
                false,
            ));
        }
    };

    if result.rows_affected() == 0 {
        return AppResponse::err(AppError::new(E_NOT_FOUND, "项目不存在", false));
    }

    let project_dir = PathBuf::from(workspace_path).join("projects").join(&id);
    if fs::try_exists(&project_dir).await.unwrap_or(false) {
        if let Err(error) = fs::remove_dir_all(&project_dir).await {
            return AppResponse::err(AppError::new(
                E_IO,
                format!("项目记录已删除，但目录清理失败: {error}"),
                false,
            ));
        }
    }

    AppResponse::ok(EmptyData {})
}