use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::State;

use crate::{
    state::AppState,
    types::{
        error_codes::{E_DB, E_WORKSPACE_INVALID},
        response::{AppError, AppResponse},
    },
};

const GENERATION_SETTINGS_ID: &str = "default";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSettings {
    pub strict_api_name_validation: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGenerationSettingsInput {
    pub strict_api_name_validation: bool,
}

#[tauri::command]
pub fn get_generation_settings(state: State<'_, AppState>) -> AppResponse<GenerationSettings> {
    tauri::async_runtime::block_on(async {
        let Some(pool) = state.get_db().await else {
            return AppResponse::err(AppError::new(
                E_WORKSPACE_INVALID,
                "工作区尚未初始化，无法读取生成设置",
                false,
            ));
        };

        match load_generation_settings(&pool).await {
            Ok(settings) => AppResponse::ok(settings),
            Err(error) => AppResponse::err(error),
        }
    })
}

#[tauri::command]
pub fn update_generation_settings(
    input: UpdateGenerationSettingsInput,
    state: State<'_, AppState>,
) -> AppResponse<GenerationSettings> {
    tauri::async_runtime::block_on(async {
        let Some(pool) = state.get_db().await else {
            return AppResponse::err(AppError::new(
                E_WORKSPACE_INVALID,
                "工作区尚未初始化，无法保存生成设置",
                false,
            ));
        };

        let settings = GenerationSettings {
            strict_api_name_validation: input.strict_api_name_validation,
        };

        match save_generation_settings(&pool, &settings).await {
            Ok(()) => AppResponse::ok(settings),
            Err(error) => AppResponse::err(error),
        }
    })
}

pub async fn load_generation_settings(pool: &SqlitePool) -> Result<GenerationSettings, AppError> {
    let row =
        sqlx::query("SELECT strict_api_name_validation FROM generation_settings WHERE id = ?")
            .bind(GENERATION_SETTINGS_ID)
            .fetch_optional(pool)
            .await
            .map_err(|error| AppError::new(E_DB, format!("无法读取生成设置: {error}"), false))?;

    let Some(row) = row else {
        return Ok(default_generation_settings());
    };

    Ok(GenerationSettings {
        strict_api_name_validation: row.get::<i64, _>("strict_api_name_validation") != 0,
    })
}

pub async fn save_generation_settings(
    pool: &SqlitePool,
    settings: &GenerationSettings,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO generation_settings (id, strict_api_name_validation, created_at, updated_at) VALUES (?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET strict_api_name_validation = excluded.strict_api_name_validation, updated_at = excluded.updated_at",
    )
    .bind(GENERATION_SETTINGS_ID)
    .bind(if settings.strict_api_name_validation { 1_i64 } else { 0_i64 })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|error| AppError::new(E_DB, format!("无法保存生成设置: {error}"), false))?;

    Ok(())
}

pub fn default_generation_settings() -> GenerationSettings {
    GenerationSettings {
        strict_api_name_validation: false,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use uuid::Uuid;

    use super::*;
    use crate::services::{db, workspace};

    #[tokio::test]
    async fn generation_settings_default_to_non_strict() {
        let (pool, workspace_root) = setup().await;

        let settings = load_generation_settings(&pool).await.unwrap();

        assert!(!settings.strict_api_name_validation);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn generation_settings_round_trip_strict_api_validation() {
        let (pool, workspace_root) = setup().await;
        let settings = GenerationSettings {
            strict_api_name_validation: true,
        };

        save_generation_settings(&pool, &settings).await.unwrap();
        let loaded = load_generation_settings(&pool).await.unwrap();

        assert!(loaded.strict_api_name_validation);

        cleanup(workspace_root).await;
    }

    async fn setup() -> (SqlitePool, PathBuf) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-settings-tests-{}", Uuid::new_v4()));
        workspace::create_standard_dirs(&workspace_root)
            .await
            .unwrap();
        let pool = db::open_or_create(&workspace_root.join("db").join("app.sqlite"))
            .await
            .unwrap();
        (pool, workspace_root)
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
