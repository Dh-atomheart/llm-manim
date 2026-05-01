use std::{
    future::Future,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sqlx::{Row, SqlitePool};

use crate::{
    commands::settings,
    services::{logging, prompt, provider, static_checker},
    types::{
        error_codes::{E_CANCELLED, E_NOT_FOUND},
        response::AppError,
    },
};

const PROVIDER_CLASSIFICATION_TIMEOUT_SECS: u64 = 500;
const PROVIDER_REWRITE_TIMEOUT_SECS: u64 = 180;
const PROVIDER_GENERATION_TIMEOUT_SECS: u64 = 500;

#[derive(Debug)]
pub struct OrchestrationResult {
    pub code: String,
    pub scene_name: String,
}

struct JobContext {
    prompt_text: String,
    provider_type: String,
    base_url: String,
    model: String,
    api_key: String,
}

fn provider_response_log_message(model_text: &str) -> String {
    format!("provider response received: bytes={}", model_text.len())
}

fn provider_error_details_log_message(error: &AppError) -> Option<String> {
    let details = error.details.as_ref()?;
    let content_type = details
        .get("contentType")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let body_excerpt = details
        .get("bodyExcerpt")
        .or_else(|| details.get("sanitizedBodyExcerpt"))
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if content_type.is_empty() && body_excerpt.is_empty() {
        return None;
    }

    Some(format!(
        "provider response details: contentType={}, bodyExcerpt={}",
        if content_type.is_empty() {
            "(missing)"
        } else {
            content_type
        },
        if body_excerpt.is_empty() {
            "(empty)"
        } else {
            body_excerpt
        }
    ))
}

fn selected_prompt_rules_log_message(selected_rules: &[String]) -> String {
    if selected_rules.is_empty() {
        return "selected prompt rules: none".to_string();
    }

    format!("selected prompt rules: {}", selected_rules.join(", "))
}

fn selected_prompt_skills_log_message(selected_skills: &[String]) -> String {
    if selected_skills.is_empty() {
        return "selected prompt skills: none".to_string();
    }

    format!("selected prompt skills: {}", selected_skills.join(", "))
}

fn selected_prompt_templates_log_message(selected_templates: &[String]) -> String {
    if selected_templates.is_empty() {
        return "selected prompt templates: none".to_string();
    }

    format!(
        "selected prompt templates: {}",
        selected_templates.join(", ")
    )
}

fn prompt_size_log_message(chars: usize) -> String {
    format!("prompt size: {chars} chars (unlimited)")
}

fn prompt_rewrite_finished_log_message(rewrite: &prompt::PromptRewrite) -> String {
    format!(
        "prompt rewrite finished: beats={}, checklist={}, refinedPromptExcerpt={}",
        rewrite.animation_beats.len(),
        rewrite.quality_checklist.len(),
        truncate_log_excerpt(&rewrite.refined_prompt, 160)
    )
}

fn truncate_log_excerpt(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim().replace(['\r', '\n'], " ");
    if trimmed.chars().count() <= max_chars {
        return trimmed;
    }
    format!("{}...", trimmed.chars().take(max_chars).collect::<String>())
}

pub async fn run_orchestration(
    pool: &SqlitePool,
    workspace_root: &Path,
    job_id: &str,
    cancel_flag: Arc<AtomicBool>,
) -> Result<OrchestrationResult, AppError> {
    run_orchestration_with_provider(
        pool,
        workspace_root,
        job_id,
        cancel_flag,
        |provider_type, base_url, api_key, model, system_prompt, user_prompt, timeout_secs| async move {
            provider::generate_with_timeout(
                &provider_type,
                &base_url,
                &api_key,
                &model,
                &system_prompt,
                &user_prompt,
                timeout_secs,
            )
            .await
        },
    )
    .await
}

async fn run_orchestration_with_provider<F, Fut>(
    pool: &SqlitePool,
    workspace_root: &Path,
    job_id: &str,
    cancel_flag: Arc<AtomicBool>,
    provider_generate: F,
) -> Result<OrchestrationResult, AppError>
where
    F: Fn(String, String, String, String, String, String, u64) -> Fut,
    Fut: Future<Output = Result<String, AppError>>,
{
    let context = load_job_context(pool, job_id).await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        "prompt build started",
    )
    .await;

    check_cancel(job_id, pool, workspace_root, &cancel_flag).await?;

    let classification_system_prompt = prompt::build_skill_classification_system_prompt();
    let classification_user_prompt =
        prompt::build_skill_classification_user_prompt(&context.prompt_text);
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "provider",
        "info",
        "provider classification request started",
    )
    .await;
    let skill_selection = match provider_generate(
        context.provider_type.clone(),
        context.base_url.clone(),
        context.api_key.clone(),
        context.model.clone(),
        classification_system_prompt,
        classification_user_prompt,
        PROVIDER_CLASSIFICATION_TIMEOUT_SECS,
    )
    .await
    {
        Ok(classification_text) => match prompt::parse_skill_classification(&classification_text) {
            Ok(selection) => selection,
            Err(error) => {
                logging::write_job_log(
                    Some(pool),
                    Some(workspace_root),
                    job_id,
                    "prompt",
                    "warning",
                    &format!("provider classification fallback: {}", error.message),
                )
                .await;
                prompt::generic_skill_selection()
            }
        },
        Err(error) => {
            logging::write_job_log(
                Some(pool),
                Some(workspace_root),
                job_id,
                "provider",
                "warning",
                &format!("provider classification request failed: {}", error.message),
            )
            .await;
            logging::write_job_log(
                Some(pool),
                Some(workspace_root),
                job_id,
                "prompt",
                "warning",
                "provider classification fallback: generic skill selection",
            )
            .await;
            prompt::generic_skill_selection()
        }
    };

    let prompt_rewrite = build_prompt_rewrite_with_retry(
        pool,
        workspace_root,
        job_id,
        &context,
        &skill_selection,
        &provider_generate,
    )
    .await;

    let generation_settings = settings::load_generation_settings(pool).await?;
    let prompt_assembly = prompt::build_prompt_assembly_from_selection_rewrite_and_settings(
        &context.prompt_text,
        &skill_selection,
        prompt_rewrite.as_ref(),
        generation_settings.strict_api_name_validation,
    );
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        "prompt build finished",
    )
    .await;
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        &selected_prompt_skills_log_message(&prompt_assembly.selected_skills),
    )
    .await;
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        &selected_prompt_rules_log_message(&prompt_assembly.selected_rules),
    )
    .await;
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        &selected_prompt_templates_log_message(&prompt_assembly.selected_templates),
    )
    .await;
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "info",
        &prompt_size_log_message(prompt_assembly.prompt_chars),
    )
    .await;

    let system_prompt = prompt_assembly.system_prompt;
    let user_prompt = prompt_assembly.user_prompt;

    check_cancel(job_id, pool, workspace_root, &cancel_flag).await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "provider",
        "info",
        "provider generation request started",
    )
    .await;
    let model_text = match provider_generate(
        context.provider_type.clone(),
        context.base_url.clone(),
        context.api_key.clone(),
        context.model.clone(),
        system_prompt.clone(),
        user_prompt.clone(),
        PROVIDER_GENERATION_TIMEOUT_SECS,
    )
    .await
    {
        Ok(text) => text,
        Err(error) => {
            logging::write_job_log(
                Some(pool),
                Some(workspace_root),
                job_id,
                "provider",
                "error",
                &format!("provider generation request failed: {}", error.message),
            )
            .await;
            if let Some(details_message) = provider_error_details_log_message(&error) {
                logging::write_job_log(
                    Some(pool),
                    Some(workspace_root),
                    job_id,
                    "provider",
                    "error",
                    &details_message,
                )
                .await;
            }
            return Err(error);
        }
    };
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "provider",
        "info",
        &provider_response_log_message(&model_text),
    )
    .await;

    check_cancel(job_id, pool, workspace_root, &cancel_flag).await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "parse",
        "info",
        "markdown parse started",
    )
    .await;
    let code = match prompt::parse_markdown_code_block(&model_text) {
        Ok(code) => code,
        Err(error) => {
            logging::write_job_log(
                Some(pool),
                Some(workspace_root),
                job_id,
                "parse",
                "error",
                &format!("markdown parse failed: {}", error.message),
            )
            .await;
            return Err(error);
        }
    };
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "parse",
        "info",
        "markdown parse finished",
    )
    .await;

    check_cancel(job_id, pool, workspace_root, &cancel_flag).await?;

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "static_check",
        "info",
        "static check started",
    )
    .await;
    let checked = match static_checker::run_static_check(
        workspace_root,
        &code,
        generation_settings.strict_api_name_validation,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            logging::write_job_log(
                Some(pool),
                Some(workspace_root),
                job_id,
                "static_check",
                "error",
                &format!("static check failed: {}", error.message),
            )
            .await;
            return Err(error);
        }
    };
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "static_check",
        "info",
        "static check finished",
    )
    .await;
    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "llm",
        "info",
        "handoff to render",
    )
    .await;

    Ok(OrchestrationResult {
        code: checked.normalized_code,
        scene_name: checked.scene_name,
    })
}

async fn build_prompt_rewrite_with_retry<F, Fut>(
    pool: &SqlitePool,
    workspace_root: &Path,
    job_id: &str,
    context: &JobContext,
    skill_selection: &prompt::SkillSelection,
    provider_generate: &F,
) -> Option<prompt::PromptRewrite>
where
    F: Fn(String, String, String, String, String, String, u64) -> Fut,
    Fut: Future<Output = Result<String, AppError>>,
{
    let rewrite_system_prompt = prompt::build_prompt_rewrite_system_prompt();
    let rewrite_user_prompt =
        prompt::build_prompt_rewrite_user_prompt(&context.prompt_text, skill_selection);

    for attempt in 1..=2 {
        logging::write_job_log(
            Some(pool),
            Some(workspace_root),
            job_id,
            "provider",
            "info",
            "provider rewrite request started",
        )
        .await;

        let rewrite_text = match provider_generate(
            context.provider_type.clone(),
            context.base_url.clone(),
            context.api_key.clone(),
            context.model.clone(),
            rewrite_system_prompt.clone(),
            rewrite_user_prompt.clone(),
            PROVIDER_REWRITE_TIMEOUT_SECS,
        )
        .await
        {
            Ok(text) => text,
            Err(error) => {
                logging::write_job_log(
                    Some(pool),
                    Some(workspace_root),
                    job_id,
                    "provider",
                    "warning",
                    &format!(
                        "provider rewrite request failed: attempt={}, {}",
                        attempt, error.message
                    ),
                )
                .await;
                continue;
            }
        };

        match prompt::parse_prompt_rewrite(&rewrite_text) {
            Ok(rewrite) => {
                logging::write_job_log(
                    Some(pool),
                    Some(workspace_root),
                    job_id,
                    "prompt",
                    "info",
                    &prompt_rewrite_finished_log_message(&rewrite),
                )
                .await;
                return Some(rewrite);
            }
            Err(error) => {
                logging::write_job_log(
                    Some(pool),
                    Some(workspace_root),
                    job_id,
                    "provider",
                    "warning",
                    &format!(
                        "provider rewrite request failed: attempt={}, {}",
                        attempt, error.message
                    ),
                )
                .await;
            }
        }
    }

    logging::write_job_log(
        Some(pool),
        Some(workspace_root),
        job_id,
        "prompt",
        "warning",
        "prompt rewrite fallback: original prompt",
    )
    .await;

    None
}

async fn check_cancel(
    job_id: &str,
    pool: &SqlitePool,
    workspace_root: &Path,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(), AppError> {
    if cancel_flag.load(Ordering::Relaxed) {
        logging::write_job_log(
            Some(pool),
            Some(workspace_root),
            job_id,
            "user_action",
            "info",
            "job cancellation observed before render handoff",
        )
        .await;
        return Err(AppError::new(E_CANCELLED, "任务已取消", false));
    }

    Ok(())
}

async fn load_job_context(pool: &SqlitePool, job_id: &str) -> Result<JobContext, AppError> {
    let row = sqlx::query(
        "SELECT j.prompt_text, p.provider_type, p.base_url, p.model, p.api_key \
         FROM prompt_jobs j \
         INNER JOIN provider_configs p ON p.id = j.provider_id \
         WHERE j.id = ?",
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| AppError::new(E_NOT_FOUND, format!("无法读取任务上下文: {error}"), false))?;

    let Some(row) = row else {
        return Err(AppError::new(
            E_NOT_FOUND,
            "任务不存在或 Provider 已丢失",
            false,
        ));
    };

    Ok(JobContext {
        prompt_text: row.get("prompt_text"),
        provider_type: row.get("provider_type"),
        base_url: row.get("base_url"),
        model: row.get("model"),
        api_key: row.get("api_key"),
    })
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::{
        services::{db, workspace},
        types::error_codes::{
            E_AUTH_401, E_CANCELLED, E_LLM_OUTPUT_INVALID, E_PROVIDER_RESPONSE_INVALID,
            E_STATIC_CHECK_FAILED,
        },
    };

    #[test]
    fn provider_response_log_message_uses_only_length_summary() {
        let model_text = "```python\nclass SecretScene(Scene):\n    pass\n```";

        let message = provider_response_log_message(model_text);

        assert_eq!(
            message,
            format!("provider response received: bytes={}", model_text.len())
        );
        assert!(!message.contains("SecretScene"));
        assert!(!message.contains("```python"));
    }

    #[test]
    fn provider_error_details_log_message_includes_content_type_and_excerpt() {
        let error = AppError::new(
            E_PROVIDER_RESPONSE_INVALID,
            "Provider returned a non-JSON HTTP response.",
            false,
        )
        .with_details(serde_json::json!({
            "contentType": "text/html",
            "bodyExcerpt": "<html>timeout</html>",
        }));

        let message = provider_error_details_log_message(&error).unwrap();

        assert_eq!(
            message,
            "provider response details: contentType=text/html, bodyExcerpt=<html>timeout</html>"
        );
    }

    #[test]
    fn selected_prompt_rules_log_message_lists_rule_names() {
        let message = selected_prompt_rules_log_message(&[
            "manim-composer/summary".to_string(),
            "manimce/latex".to_string(),
        ]);

        assert_eq!(
            message,
            "selected prompt rules: manim-composer/summary, manimce/latex"
        );
    }

    #[test]
    fn prompt_metadata_log_messages_handle_empty_and_non_empty_values() {
        assert_eq!(
            selected_prompt_skills_log_message(&["manim-composer/SKILL.md".to_string()]),
            "selected prompt skills: manim-composer/SKILL.md"
        );
        assert_eq!(
            selected_prompt_templates_log_message(&["manimce/basic_scene".to_string()]),
            "selected prompt templates: manimce/basic_scene"
        );
        assert_eq!(
            prompt_size_log_message(120),
            "prompt size: 120 chars (unlimited)"
        );
    }

    #[tokio::test]
    async fn run_orchestration_accepts_valid_provider_code_block() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let result = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async { Ok(valid_markdown_block().to_string()) },
        )
        .await
        .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert!(result.code.contains("class Demo(Scene):"));

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        let selected_rules_index = log_messages
            .iter()
            .position(|message| message == "selected prompt rules: none")
            .expect("selected prompt rules log should be written");
        let provider_request_index = log_messages
            .iter()
            .position(|message| message == "provider generation request started")
            .expect("provider request log should be written");

        assert!(selected_rules_index < provider_request_index);
        assert!(log_messages
            .iter()
            .any(|message| message.starts_with("selected prompt skills: ")));
        assert!(log_messages
            .iter()
            .any(|message| message == "selected prompt templates: none"));
        assert!(log_messages
            .iter()
            .any(|message| message.starts_with("prompt size: ")));
        assert!(log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_uses_skill_classification_before_generation() {
        let (pool, workspace_root, job_id) = setup_job_context().await;
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let result = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            {
                let call_count = Arc::clone(&call_count);
                move |_, _, _, _, _, _, _| {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        let call_index = call_count.fetch_add(1, Ordering::SeqCst);
                        if call_index == 0 {
                            Ok(r#"{"taskKinds":["graph"],"rules":["manimce/axes","manimce/graphing","manimce/positioning"],"templates":["manimce/basic_scene"],"rationale":"graph request"}"#.to_string())
                        } else if call_index == 1 {
                            Ok(valid_prompt_rewrite_json().to_string())
                        } else {
                            Ok(valid_markdown_block().to_string())
                        }
                    }
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages.iter().any(|message| {
            message == "selected prompt rules: manimce/axes, manimce/graphing, manimce/positioning"
        }));
        assert!(log_messages
            .iter()
            .any(|message| message == "selected prompt templates: manimce/basic_scene"));
        assert!(log_messages
            .iter()
            .any(|message| message.starts_with("prompt rewrite finished: beats=3")));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_retries_prompt_rewrite_once_then_generates() {
        let (pool, workspace_root, job_id) = setup_job_context().await;
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let timeouts = Arc::new(std::sync::Mutex::new(Vec::new()));

        let result = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            {
                let call_count = Arc::clone(&call_count);
                let timeouts = Arc::clone(&timeouts);
                move |_, _, _, _, _, _, timeout_secs| {
                    let call_count = Arc::clone(&call_count);
                    let timeouts = Arc::clone(&timeouts);
                    async move {
                        timeouts.lock().unwrap().push(timeout_secs);
                        let call_index = call_count.fetch_add(1, Ordering::SeqCst);
                        match call_index {
                            0 => Ok(r#"{"taskKinds":["geometry"],"rules":["manimce/shapes","manimce/grouping"],"templates":["manimce/basic_scene"],"rationale":"geometry"}"#.to_string()),
                            1 => Err(AppError::new(
                                E_PROVIDER_RESPONSE_INVALID,
                                "Provider returned a non-JSON HTTP response.",
                                false,
                            )),
                            2 => Ok(valid_prompt_rewrite_json().to_string()),
                            _ => Ok(valid_markdown_block().to_string()),
                        }
                    }
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
        assert_eq!(
            *timeouts.lock().unwrap(),
            vec![
                PROVIDER_CLASSIFICATION_TIMEOUT_SECS,
                PROVIDER_REWRITE_TIMEOUT_SECS,
                PROVIDER_REWRITE_TIMEOUT_SECS,
                PROVIDER_GENERATION_TIMEOUT_SECS,
            ]
        );

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("provider rewrite request failed: attempt=1")));
        assert!(log_messages
            .iter()
            .any(|message| message.starts_with("prompt rewrite finished: beats=3")));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_falls_back_to_original_prompt_when_rewrite_fails_twice() {
        let (pool, workspace_root, job_id) = setup_job_context().await;
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let result = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            {
                let call_count = Arc::clone(&call_count);
                move |_, _, _, _, _, _, _| {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        let call_index = call_count.fetch_add(1, Ordering::SeqCst);
                        match call_index {
                            0 => Ok(r#"{"taskKinds":["general"],"rules":[],"templates":[],"rationale":"general"}"#.to_string()),
                            1 | 2 => Ok("not json".to_string()),
                            _ => Ok(valid_markdown_block().to_string()),
                        }
                    }
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert_eq!(call_count.load(Ordering::SeqCst), 4);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message == "prompt rewrite fallback: original prompt"));
        assert!(log_messages
            .iter()
            .any(|message| message == "provider generation request started"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_falls_back_when_classification_provider_fails() {
        let (pool, workspace_root, job_id) = setup_job_context().await;
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let result = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            {
                let call_count = Arc::clone(&call_count);
                move |_, _, _, _, _, _, _| {
                    let call_count = Arc::clone(&call_count);
                    async move {
                        let call_index = call_count.fetch_add(1, Ordering::SeqCst);
                        if call_index == 0 {
                            Err(AppError::new(
                                E_PROVIDER_RESPONSE_INVALID,
                                "Provider returned a non-JSON HTTP response.",
                                false,
                            ))
                        } else if call_index == 1 {
                            Ok(valid_prompt_rewrite_json().to_string())
                        } else {
                            Ok(valid_markdown_block().to_string())
                        }
                    }
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("provider classification request failed")));
        assert!(log_messages
            .iter()
            .any(|message| message == "provider classification fallback: generic skill selection"));
        assert!(log_messages
            .iter()
            .any(|message| message == "provider generation request started"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_rejects_missing_markdown_block() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async { Ok("no code block here".to_string()) },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_LLM_OUTPUT_INVALID);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("markdown parse failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_rejects_multiple_markdown_blocks() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async { Ok(multiple_markdown_blocks().to_string()) },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_LLM_OUTPUT_INVALID);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("markdown parse failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_rejects_manimgl_code() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async { Ok(manimgl_markdown_block().to_string()) },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_STATIC_CHECK_FAILED);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("static check failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_rejects_dangerous_code() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async { Ok(dangerous_markdown_block().to_string()) },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_STATIC_CHECK_FAILED);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("static check failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_propagates_provider_auth_errors() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async {
                Err(AppError::new(
                    E_AUTH_401,
                    "Provider 鉴权失败，请检查 API Key",
                    false,
                ))
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_AUTH_401);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("provider generation request failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_propagates_provider_response_invalid_errors() {
        let (pool, workspace_root, job_id) = setup_job_context().await;

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            Arc::new(AtomicBool::new(false)),
            |_, _, _, _, _, _, _| async {
                Err(AppError::new(
                    E_PROVIDER_RESPONSE_INVALID,
                    "Provider 返回了无法解析的响应结构",
                    false,
                ))
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_PROVIDER_RESPONSE_INVALID);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message.contains("provider generation request failed")));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_orchestration_stops_when_cancelled_before_provider_request() {
        let (pool, workspace_root, job_id) = setup_job_context().await;
        let cancel_flag = Arc::new(AtomicBool::new(true));

        let error = run_orchestration_with_provider(
            &pool,
            &workspace_root,
            &job_id,
            cancel_flag,
            |_, _, _, _, _, _, _| async {
                panic!("provider should not be called when cancellation is observed")
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, E_CANCELLED);

        let log_messages = load_job_log_messages(&pool, &job_id).await;
        assert!(log_messages
            .iter()
            .any(|message| message == "job cancellation observed before render handoff"));
        assert!(!log_messages
            .iter()
            .any(|message| message == "handoff to render"));
        assert_no_generated_scene(&workspace_root, &job_id);

        cleanup(workspace_root).await;
    }

    async fn setup_job_context() -> (SqlitePool, PathBuf, String) {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-orchestrator-tests-{}", Uuid::new_v4()));
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
        .bind("Explain quadratic formula")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        (pool, workspace_root, job_id)
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

    fn assert_no_generated_scene(workspace_root: &Path, job_id: &str) {
        assert!(!workspace_root
            .join("jobs")
            .join(job_id)
            .join("generated_scene.py")
            .exists());
    }

    fn valid_markdown_block() -> &'static str {
        concat!(
            "```python\n",
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        title = Text('Demo')\n",
            "        self.play(Write(title))\n",
            "        self.wait()\n",
            "```\n",
        )
    }

    fn valid_prompt_rewrite_json() -> &'static str {
        r#"{
            "refinedPrompt": "Create a centered geometric proof animation with clear spacing.",
            "contentPlan": "Preserve the original proof goal and variable meanings.",
            "visualDesign": "Use a centered VGroup, clear whitespace, semantic colors, and no overlapping labels.",
            "animationBeats": [
                {"action":"show base diagram","focus":"main square","pause":"0.5s"},
                {"action":"reveal partitions","focus":"area pieces","pause":"0.5s"},
                {"action":"write final relation","focus":"formula","pause":"1s"}
            ],
            "labelingPlan": "Bind labels to shapes and keep them outside crowded corners.",
            "codeGuidance": "Group geometry in VGroup(...).move_to(ORIGIN); avoid lower-left anchoring.",
            "qualityChecklist": ["main diagram centered","labels do not overlap","formula does not cover geometry"]
        }"#
    }

    fn multiple_markdown_blocks() -> &'static str {
        concat!(
            "```python\n",
            "from manim import *\n",
            "class A(Scene):\n",
            "    def construct(self):\n",
            "        self.wait()\n",
            "```\n\n",
            "```python\n",
            "from manim import *\n",
            "class B(Scene):\n",
            "    def construct(self):\n",
            "        self.wait()\n",
            "```\n",
        )
    }

    fn manimgl_markdown_block() -> &'static str {
        concat!(
            "```python\n",
            "from manim import *\n",
            "from manimlib import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        self.wait()\n",
            "```\n",
        )
    }

    fn dangerous_markdown_block() -> &'static str {
        concat!(
            "```python\n",
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        open('secret.txt')\n",
            "        self.wait()\n",
            "```\n",
        )
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = tokio::fs::remove_dir_all(workspace_root).await;
    }
}
