use reqwest::{Client, StatusCode, Url};
use serde_json::{json, Value};

use crate::{
    services::redact,
    types::{
        error_codes::{
            E_AUTH_401, E_NET_TIMEOUT, E_PROVIDER_ERROR, E_PROVIDER_RESPONSE_INVALID,
            E_VALIDATION,
        },
        response::AppError,
    },
};

const TEST_TIMEOUT_SECS: u64 = 30;

pub async fn test_provider(
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<String, AppError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(TEST_TIMEOUT_SECS))
        .build()
        .map_err(|error| AppError::new(E_PROVIDER_ERROR, format!("无法创建 HTTP 客户端: {error}"), false))?;

    match provider_type {
        "openai_compatible" => test_openai_compatible(&client, base_url, api_key, model).await,
        "anthropic_compatible" => {
            test_anthropic_compatible(&client, base_url, api_key, model).await
        }
        _ => Err(AppError::new(E_VALIDATION, "不支持的 Provider 类型", false)),
    }
}

pub fn validate_provider_type(provider_type: &str) -> Result<(), AppError> {
    match provider_type {
        "openai_compatible" | "anthropic_compatible" => Ok(()),
        _ => Err(AppError::new(E_VALIDATION, "不支持的 Provider 类型", false)),
    }
}

pub fn validate_base_url(base_url: &str) -> Result<(), AppError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(E_VALIDATION, "Base URL 不能为空", false));
    }

    let parsed = Url::parse(trimmed)
        .map_err(|_| AppError::new(E_VALIDATION, "Base URL 格式无效", false))?;

    match parsed.scheme() {
        "https" => Ok(()),
        "http" => match parsed.host_str() {
            Some("localhost") | Some("127.0.0.1") | Some("::1") => Ok(()),
            _ => Err(AppError::new(
                E_VALIDATION,
                "Base URL 必须使用 HTTPS，或使用本地测试地址",
                false,
            )),
        },
        _ => Err(AppError::new(
            E_VALIDATION,
            "Base URL 必须使用 HTTPS，或使用本地测试地址",
            false,
        )),
    }
}

async fn test_openai_compatible(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<String, AppError> {
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "messages": [{ "role": "user", "content": "Reply with OK only." }],
            "temperature": 0.0,
            "stream": false,
        }))
        .send()
        .await
        .map_err(|error| map_request_error("openai_compatible", &endpoint, api_key, error))?;

    handle_response("openai_compatible", &endpoint, api_key, response, parse_openai_text).await
}

async fn test_anthropic_compatible(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<String, AppError> {
    let endpoint = format!("{}/messages", base_url.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": model,
            "messages": [{ "role": "user", "content": "Reply with OK only." }],
            "max_tokens": 16,
            "temperature": 0.0,
        }))
        .send()
        .await
        .map_err(|error| map_request_error("anthropic_compatible", &endpoint, api_key, error))?;

    handle_response(
        "anthropic_compatible",
        &endpoint,
        api_key,
        response,
        parse_anthropic_text,
    )
    .await
}

async fn handle_response(
    provider_type: &str,
    endpoint: &str,
    api_key: &str,
    response: reqwest::Response,
    parser: fn(&Value) -> Option<String>,
) -> Result<String, AppError> {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let sanitized_text = redact::truncate(&redact::redact(&text, &[api_key]), 240);

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(AppError::new(E_AUTH_401, "Provider 鉴权失败，请检查 API Key", false)
            .with_details(json!({
                "providerType": provider_type,
                "httpStatus": status.as_u16(),
                "requestUrl": redact::sanitize_url(endpoint),
                "sanitizedBodyExcerpt": sanitized_text,
            })));
    }

    if !status.is_success() {
        return Err(AppError::new(E_PROVIDER_ERROR, "Provider 返回了非成功状态", true).with_details(
            json!({
                "providerType": provider_type,
                "httpStatus": status.as_u16(),
                "requestUrl": redact::sanitize_url(endpoint),
                "sanitizedBodyExcerpt": sanitized_text,
            }),
        ));
    }

    let payload: Value = serde_json::from_str(&text).map_err(|_| {
        AppError::new(E_PROVIDER_RESPONSE_INVALID, "Provider 响应不是有效 JSON", false)
            .with_details(json!({
                "providerType": provider_type,
                "requestUrl": redact::sanitize_url(endpoint),
                "sanitizedBodyExcerpt": sanitized_text,
            }))
    })?;

    parser(&payload).filter(|content| !content.trim().is_empty()).ok_or_else(|| {
        AppError::new(
            E_PROVIDER_RESPONSE_INVALID,
            "Provider 响应缺少可读文本内容",
            false,
        )
        .with_details(json!({
            "providerType": provider_type,
            "requestUrl": redact::sanitize_url(endpoint),
            "sanitizedBodyExcerpt": redact::truncate(&redact::redact(&payload.to_string(), &[api_key]), 240),
        }))
    })
}

fn parse_openai_text(payload: &Value) -> Option<String> {
    payload
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
        .map(ToString::to_string)
}

fn parse_anthropic_text(payload: &Value) -> Option<String> {
    payload
        .get("content")?
        .as_array()?
        .iter()
        .find_map(|block| match (block.get("type")?.as_str(), block.get("text")?.as_str()) {
            (Some("text"), Some(text)) => Some(text.to_string()),
            _ => None,
        })
}

fn map_request_error(
    provider_type: &str,
    endpoint: &str,
    api_key: &str,
    error: reqwest::Error,
) -> AppError {
    if error.is_timeout() {
        return AppError::new(E_NET_TIMEOUT, "连接测试超时，请稍后重试", true).with_details(
            json!({
                "providerType": provider_type,
                "requestUrl": redact::sanitize_url(endpoint),
            }),
        );
    }

    let sanitized = redact::truncate(&redact::redact(&error.to_string(), &[api_key]), 200);
    AppError::new(E_PROVIDER_ERROR, "无法连接到 Provider 服务", true).with_details(json!({
        "providerType": provider_type,
        "requestUrl": redact::sanitize_url(endpoint),
        "sanitizedBodyExcerpt": sanitized,
    }))
}