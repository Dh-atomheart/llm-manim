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
pub fn list_provider_configs(
    state: State<'_, AppState>,
) -> AppResponse<Vec<ProviderConfigSummary>> {
    tauri::async_runtime::block_on(list_provider_configs_inner(state.inner()))
}

async fn list_provider_configs_inner(state: &AppState) -> AppResponse<Vec<ProviderConfigSummary>> {
    let Some(pool) = state.get_db().await else {
        return AppResponse::ok(Vec::new());
    };

    let rows = match sqlx::query(
        "SELECT id, name, provider_type, base_url, model, created_at, updated_at \
         FROM provider_configs \
         WHERE deleted_at IS NULL \
         ORDER BY updated_at DESC",
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
        Some(id) => match load_active_provider_record(&pool, id).await {
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

    let save_result = if existing.is_some() {
        sqlx::query(
            "UPDATE provider_configs \
             SET name = ?, provider_type = ?, base_url = ?, model = ?, api_key = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(name)
        .bind(provider_type)
        .bind(base_url)
        .bind(model)
        .bind(&api_key)
        .bind(&updated_at)
        .bind(&provider_id)
        .execute(&pool)
        .await
    } else {
        sqlx::query(
            "INSERT INTO provider_configs (id, name, provider_type, base_url, model, api_key, created_at, updated_at, deleted_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)",
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
    };

    match save_result {
        Ok(result) if result.rows_affected() > 0 => {}
        Ok(_) => {
            return AppResponse::err(AppError::new(E_NOT_FOUND, "Provider 不存在", false));
        }
        Err(error) => {
            return AppResponse::err(AppError::new(
                E_DB,
                format!("无法保存 Provider 配置: {error}"),
                false,
            ));
        }
    }

    let sanitized_message = redact::redact(
        &format!("Provider 配置已保存: {name} / {provider_type} / {base_url} / {model}"),
        &[&api_key],
    );
    let workspace_root = workspace_root_from_state(state).await;
    logging::write_provider_log(
        Some(&pool),
        workspace_root.as_deref(),
        "info",
        &sanitized_message,
    )
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

    if let Err(error) = load_active_provider_record(&pool, &id).await {
        return AppResponse::err(error);
    }

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

    let now = Utc::now().to_rfc3339();
    let result = match sqlx::query(
        "UPDATE provider_configs \
         SET deleted_at = ?, updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
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
        Some(id) => match load_active_provider_record(&pool, id).await {
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
    logging::write_provider_log(
        Some(&pool),
        workspace_root.as_deref(),
        "info",
        &start_message,
    )
    .await;

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
            logging::write_provider_log(
                Some(&pool),
                workspace_root.as_deref(),
                "error",
                &log_message,
            )
            .await;

            let details = error.details.clone().unwrap_or_else(|| json!({}));
            AppResponse::err(
                AppError::new(error.code, sanitized_summary, error.retryable).with_details(details),
            )
        }
    }
}

async fn load_active_provider_record(
    pool: &sqlx::SqlitePool,
    id: &str,
) -> Result<ProviderConfigRecord, AppError> {
    let row = sqlx::query(
        "SELECT id, name, provider_type, base_url, model, api_key, created_at, updated_at \
         FROM provider_configs \
         WHERE id = ? AND deleted_at IS NULL",
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

async fn provider_has_active_jobs(
    pool: &sqlx::SqlitePool,
    provider_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "SELECT 1 FROM prompt_jobs WHERE provider_id = ? AND state IN ('queued', 'running') LIMIT 1",
    )
    .bind(provider_id)
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}

async fn workspace_root_from_state(state: &AppState) -> Option<PathBuf> {
    state.get_workspace_path().await.map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use chrono::Utc;
    use sqlx::{Row, SqlitePool};
    use uuid::Uuid;

    use super::*;
    use crate::{
        services::{db, workspace},
        types::error_codes::{E_NOT_FOUND, E_PROVIDER_IN_USE},
    };

    #[tokio::test]
    async fn save_and_list_provider_do_not_leak_api_key() {
        let (state, workspace_root) = setup_test_state().await;

        let save_response = save_provider_config_inner(
            SaveProviderConfigInput {
                id: None,
                name: "DeepSeek".to_string(),
                provider_type: "openai_compatible".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                model: "deepseek-v4-pro".to_string(),
                api_key: Some("sk-provider-secret".to_string()),
            },
            &state,
        )
        .await;

        assert!(save_response.ok);
        let list_response = list_provider_configs_inner(&state).await;
        assert!(list_response.ok);

        let serialized = serde_json::to_string(&list_response).unwrap();
        assert!(!serialized.contains("sk-provider-secret"));
        assert!(!serialized.contains("apiKey"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn delete_provider_rejects_when_queued_or_running_job_references_it() {
        for job_state in ["queued", "running"] {
            let (state, workspace_root) = setup_test_state().await;
            let pool = state.get_db().await.unwrap();
            let provider_id = create_provider(&state).await;
            let project_id = insert_project(&pool).await;
            insert_prompt_job(&pool, &project_id, &provider_id, job_state).await;

            let response = delete_provider_config_inner(provider_id.clone(), &state).await;

            assert!(!response.ok);
            assert_eq!(response.error.unwrap().code, E_PROVIDER_IN_USE);

            let deleted_at: Option<String> =
                sqlx::query("SELECT deleted_at FROM provider_configs WHERE id = ?")
                    .bind(&provider_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
                    .get("deleted_at");
            assert!(deleted_at.is_none());

            cleanup(workspace_root).await;
        }
    }

    #[tokio::test]
    async fn delete_provider_soft_deletes_and_preserves_historical_job_reference() {
        let (state, workspace_root) = setup_test_state().await;
        let pool = state.get_db().await.unwrap();
        let provider_id = create_provider(&state).await;
        let project_id = insert_project(&pool).await;
        let job_id = insert_prompt_job(&pool, &project_id, &provider_id, "succeeded").await;

        let response = delete_provider_config_inner(provider_id.clone(), &state).await;

        assert!(response.ok);
        let deleted_at: Option<String> =
            sqlx::query("SELECT deleted_at FROM provider_configs WHERE id = ?")
                .bind(&provider_id)
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("deleted_at");
        assert!(deleted_at.is_some());

        let preserved_provider_id: String =
            sqlx::query("SELECT provider_id FROM prompt_jobs WHERE id = ?")
                .bind(&job_id)
                .fetch_one(&pool)
                .await
                .unwrap()
                .get("provider_id");
        assert_eq!(preserved_provider_id, provider_id);

        let list_response = list_provider_configs_inner(&state).await;
        assert!(list_response.ok);
        assert!(list_response.data.unwrap().is_empty());

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn saving_deleted_provider_returns_not_found_instead_of_reviving_it() {
        let (state, workspace_root) = setup_test_state().await;
        let provider_id = create_provider(&state).await;
        let delete_response = delete_provider_config_inner(provider_id.clone(), &state).await;
        assert!(delete_response.ok);

        let save_response = save_provider_config_inner(
            SaveProviderConfigInput {
                id: Some(provider_id),
                name: "Renamed".to_string(),
                provider_type: "openai_compatible".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                model: "deepseek-v4-pro".to_string(),
                api_key: Some("sk-new-secret".to_string()),
            },
            &state,
        )
        .await;

        assert!(!save_response.ok);
        assert_eq!(save_response.error.unwrap().code, E_NOT_FOUND);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn testing_deleted_provider_reference_returns_not_found() {
        let (state, workspace_root) = setup_test_state().await;
        let provider_id = create_provider(&state).await;
        let delete_response = delete_provider_config_inner(provider_id.clone(), &state).await;
        assert!(delete_response.ok);

        let test_response = test_provider_config_inner(
            TestProviderConfigInput {
                id: Some(provider_id),
                provider_type: None,
                base_url: None,
                model: None,
                api_key: None,
            },
            &state,
        )
        .await;

        assert!(!test_response.ok);
        assert_eq!(test_response.error.unwrap().code, E_NOT_FOUND);

        cleanup(workspace_root).await;
    }

    async fn setup_test_state() -> (AppState, PathBuf) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-provider-tests-{}", Uuid::new_v4()));
        workspace::create_standard_dirs(&workspace_root)
            .await
            .unwrap();

        let pool = db::open_or_create(&workspace_root.join("db").join("app.sqlite"))
            .await
            .unwrap();
        let state = AppState::default();
        state
            .set_workspace(workspace_root.to_string_lossy().into_owned(), pool)
            .await;

        (state, workspace_root)
    }

    async fn create_provider(state: &AppState) -> String {
        let response = save_provider_config_inner(
            SaveProviderConfigInput {
                id: None,
                name: "Provider".to_string(),
                provider_type: "openai_compatible".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
                model: "deepseek-v4-pro".to_string(),
                api_key: Some("sk-test-provider".to_string()),
            },
            state,
        )
        .await;

        assert!(response.ok);
        response.data.unwrap().id
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
            "INSERT INTO prompt_jobs (id, project_id, provider_id, prompt_text, state, error_code, error_summary, suggestion, retry_of_job_id, created_at, started_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, ?, NULL, NULL)",
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

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
