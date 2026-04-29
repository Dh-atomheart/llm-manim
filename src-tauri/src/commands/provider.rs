use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::{
    services::{logging, provider as provider_service, redact},
    state::AppState,
    types::{
        error_codes::{E_DB, E_NOT_FOUND, E_PROVIDER_IN_USE, E_VALIDATION, E_WORKSPACE_INVALID},
        response::{AppError, AppResponse},
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderConfigInput {
    id: Option<String>,
    name: String,
    provider_type: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProviderConfigInput {
    id: Option<String>,
    provider_type: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigSummary {
    id: String,
    name: String,
    provider_type: String,
    base_url: String,
    model: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProviderConfig {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProviderConfigResult {
    deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    reachable: bool,
    model_accepted: bool,
    message: String,
}

struct ProviderConfigRecord {
    provider_type: String,
    base_url: String,
    model: String,
    api_key: String,
    created_at: String,
}

#[tauri::command]
pub fn list_provider_configs(state: State<'_, AppState>) -> AppResponse<Vec<ProviderConfigSummary>> {
    tauri::async_runtime::block_on(list_provider_configs_inner(state.inner()))
}

async fn list_provider_configs_inner(state: &AppState) -> AppResponse<Vec<ProviderConfigSummary>> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::ok(Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT id, name, provider_type, base_url, model, created_at, updated_at FROM provider_configs ORDER BY updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法读取 Provider 列表: {error}"),
                false,
            ));
        }
    };

    AppResponse::ok(
        rows.into_iter()
            .map(|row| ProviderConfigSummary {
                id: row.get("id"),
                name: row.get("name"),
                provider_type: row.get("provider_type"),
                base_url: row.get("base_url"),
                model: row.get("model"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect(),
    )
}

#[tauri::command]
pub fn save_provider_config(
    input: SaveProviderConfigInput,
    state: State<'_, AppState>,
) -> AppResponse<SavedProviderConfig> {
    tauri::async_runtime::block_on(save_provider_config_inner(input, state.inner()))
}

async fn save_provider_config_inner(
    input: SaveProviderConfigInput,
    state: &AppState,
) -> AppResponse<SavedProviderConfig> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };

    let name = input.name.trim();
    let provider_type = input.provider_type.trim();
    let base_url = input.base_url.trim();
    let model = input.model.trim();

    if name.is_empty() {
        return AppResponse::err(AppError::new(E_VALIDATION, "Provider 名称不能为空", false));
    }
    if model.is_empty() {
        return AppResponse::err(AppError::new(E_VALIDATION, "模型 ID 不能为空", false));
    }
    if let Err(error) = provider_service::validate_provider_type(provider_type) {
        return AppResponse::err(error);
    }
    if let Err(error) = provider_service::validate_base_url(base_url) {
        return AppResponse::err(error);
    }

    let existing = match input.id.as_deref() {
        Some(id) => match load_provider_record(&pool, id).await {
            Ok(record) => Some(record),
            Err(error) => return AppResponse::err(error),
        },
        None => None,
    };

    let api_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| existing.as_ref().map(|record| record.api_key.clone()));

    let Some(api_key) = api_key else {
        return AppResponse::err(AppError::new(E_VALIDATION, "API Key 不能为空", false));
    };

    let provider_id = input
        .id
        .clone()
        .unwrap_or_else(|| format!("provider_{}", Uuid::new_v4()));
    let created_at = existing
        .as_ref()
        .map(|record| record.created_at.clone())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let updated_at = Utc::now().to_rfc3339();

    if let Err(error) = sqlx::query(
        "INSERT INTO provider_configs (id, name, provider_type, base_url, model, api_key, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, provider_type = excluded.provider_type, base_url = excluded.base_url, model = excluded.model, api_key = excluded.api_key, updated_at = excluded.updated_at",
    )
    .bind(&provider_id)
    .bind(name)
    .bind(provider_type)
    .bind(base_url)
    .bind(model)
    .bind(&api_key)
    .bind(&created_at)
    .bind(&updated_at)
    .execute(&pool)
    .await
    {
        return AppResponse::err(AppError::new(
            E_DB,
            format!("无法保存 Provider 配置: {error}"),
            false,
        ));
    }

    let sanitized_message = redact::redact(
        &format!("Provider 配置已保存: {name} / {provider_type} / {base_url} / {model}"),
        &[&api_key],
    );
    let workspace_root = workspace_root_from_state(state).await;
    logging::write_provider_log(Some(&pool), workspace_root.as_deref(), "info", &sanitized_message)
        .await;

    AppResponse::ok(SavedProviderConfig { id: provider_id })
}

#[tauri::command]
pub fn delete_provider_config(
    id: String,
    state: State<'_, AppState>,
) -> AppResponse<DeleteProviderConfigResult> {
    tauri::async_runtime::block_on(delete_provider_config_inner(id, state.inner()))
}

async fn delete_provider_config_inner(
    id: String,
    state: &AppState,
) -> AppResponse<DeleteProviderConfigResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };

    match provider_has_active_jobs(&pool, &id).await {
        Ok(true) => {
            return AppResponse::err(AppError::new(
                E_PROVIDER_IN_USE,
                "该 Provider 仍被 queued/running 任务引用，暂时不能删除",
                false,
            ));
        }
        Ok(false) => {}
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法检查 Provider 使用状态: {error}"),
                false,
            ));
        }
    }

    let result = match sqlx::query("DELETE FROM provider_configs WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
    {
        Ok(result) => result,
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法删除 Provider 配置: {error}"),
                false,
            ));
        }
    };

    if result.rows_affected() == 0 {
        return AppResponse::err(AppError::new(E_NOT_FOUND, "Provider 不存在", false));
    }

    let workspace_root = workspace_root_from_state(state).await;
    logging::write_provider_log(
        Some(&pool),
        workspace_root.as_deref(),
        "info",
        &format!("Provider 配置已删除: {id}"),
    )
    .await;

    AppResponse::ok(DeleteProviderConfigResult { deleted: true })
}

#[tauri::command]
pub fn test_provider_config(
    input: TestProviderConfigInput,
    state: State<'_, AppState>,
) -> AppResponse<ProviderTestResult> {
    tauri::async_runtime::block_on(test_provider_config_inner(input, state.inner()))
}

async fn test_provider_config_inner(
    input: TestProviderConfigInput,
    state: &AppState,
) -> AppResponse<ProviderTestResult> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::err(AppError::new(
            E_WORKSPACE_INVALID,
            "工作区尚未初始化",
            false,
        ));
    };

    let existing = match input.id.as_deref() {
        Some(id) => match load_provider_record(&pool, id).await {
            Ok(record) => Some(record),
            Err(error) => return AppResponse::err(error),
        },
        None => None,
    };

    let provider_type = input
        .provider_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| existing.as_ref().map(|record| record.provider_type.clone()));
    let base_url = input
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| existing.as_ref().map(|record| record.base_url.clone()));
    let model = input
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| existing.as_ref().map(|record| record.model.clone()));
    let api_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| existing.as_ref().map(|record| record.api_key.clone()));

    let (Some(provider_type), Some(base_url), Some(model), Some(api_key)) =
        (provider_type, base_url, model, api_key)
    else {
        return AppResponse::err(AppError::new(
            E_VALIDATION,
            "测试连接需要 Provider 类型、Base URL、模型 ID 和 API Key",
            false,
        ));
    };

    if let Err(error) = provider_service::validate_provider_type(&provider_type) {
        return AppResponse::err(error);
    }
    if let Err(error) = provider_service::validate_base_url(&base_url) {
        return AppResponse::err(error);
    }

    let workspace_root = workspace_root_from_state(state).await;
    let start_message = redact::redact(
        &format!("开始测试 Provider 连接: {provider_type} / {base_url} / {model}"),
        &[&api_key],
    );
    logging::write_provider_log(Some(&pool), workspace_root.as_deref(), "info", &start_message).await;

    match provider_service::test_provider(&provider_type, &base_url, &api_key, &model).await {
        Ok(content) => {
            let success_message = redact::redact(
                &format!("Provider 连接测试成功: {}", redact::truncate(&content, 80)),
                &[&api_key],
            );
            logging::write_provider_log(
                Some(&pool),
                workspace_root.as_deref(),
                "info",
                &success_message,
            )
            .await;

            AppResponse::ok(ProviderTestResult {
                reachable: true,
                model_accepted: true,
                message: "连接测试成功".to_string(),
            })
        }
        Err(error) => {
            let sanitized_summary = redact::redact(&error.message, &[&api_key]);
            let suggestion = match error.code.as_str() {
                "E_AUTH_401" => "请检查 API Key 是否正确，或确认账户是否有模型访问权限。",
                "E_NET_TIMEOUT" => "请检查网络连通性，或稍后重试。",
                _ => "请检查 Base URL、模型 ID 和 Provider 兼容性。",
            };
            let log_message = format!(
                "Provider 测试失败。原因：{sanitized_summary} 影响：当前 Provider 无法用于后续任务。建议动作：{suggestion}"
            );
            logging::write_provider_log(Some(&pool), workspace_root.as_deref(), "error", &log_message)
                .await;

            let details = error.details.clone().unwrap_or_else(|| json!({}));
            AppResponse::err(
                AppError::new(error.code, sanitized_summary, error.retryable).with_details(details),
            )
        }
    }
}

async fn load_provider_record(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<ProviderConfigRecord, AppError> {
    let row = sqlx::query(
        "SELECT id, name, provider_type, base_url, model, api_key, created_at, updated_at FROM provider_configs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| AppError::new(E_DB, format!("无法读取 Provider 配置: {error}"), false))?;

    let Some(row) = row else {
        return Err(AppError::new(E_NOT_FOUND, "Provider 不存在", false));
    };

    Ok(ProviderConfigRecord {
        provider_type: row.get("provider_type"),
        base_url: row.get("base_url"),
        model: row.get("model"),
        api_key: row.get("api_key"),
        created_at: row.get("created_at"),
    })
}

async fn provider_has_active_jobs(pool: &sqlx::SqlitePool, provider_id: &str) -> Result<bool, sqlx::Error> {
    for table_name in ["prompt_jobs", "jobs"] {
        let exists = sqlx::query("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ? LIMIT 1")
            .bind(table_name)
            .fetch_optional(pool)
            .await?
            .is_some();

        if !exists {
            continue;
        }

        let statement = format!(
            "SELECT 1 FROM {table_name} WHERE provider_id = ? AND state IN ('queued', 'running') LIMIT 1"
        );
        let in_use = sqlx::query(&statement)
            .bind(provider_id)
            .fetch_optional(pool)
            .await?
            .is_some();

        if in_use {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn workspace_root_from_state(state: &AppState) -> Option<PathBuf> {
    state.get_workspace_path().await.map(PathBuf::from)
}