use reqwest::{header::CONTENT_TYPE, Client, StatusCode, Url};
use serde_json::{json, Value};

use crate::{
    services::redact,
    types::{
        error_codes::{
            E_AUTH_401, E_NET_TIMEOUT, E_PROVIDER_ERROR, E_PROVIDER_RESPONSE_INVALID, E_VALIDATION,
        },
        response::AppError,
    },
};

const GENERATE_TIMEOUT_SECS: u64 = 500;
const TEST_TIMEOUT_SECS: u64 = 30;

pub async fn test_provider(
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<String, AppError> {
    let client = build_client(TEST_TIMEOUT_SECS)?;

    match provider_type {
        "openai_compatible" => test_openai_compatible(&client, base_url, api_key, model).await,
        "anthropic_compatible" => {
            test_anthropic_compatible(&client, base_url, api_key, model).await
        }
        _ => Err(AppError::new(E_VALIDATION, "不支持的 Provider 类型", false)),
    }
}

#[allow(dead_code)]
pub async fn generate(
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, AppError> {
    generate_with_timeout(
        provider_type,
        base_url,
        api_key,
        model,
        system_prompt,
        user_prompt,
        GENERATE_TIMEOUT_SECS,
    )
    .await
}

pub async fn generate_with_timeout(
    provider_type: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    timeout_secs: u64,
) -> Result<String, AppError> {
    let client = build_client(timeout_secs)?;
    match provider_type {
        "openai_compatible" => {
            generate_openai_compatible(
                &client,
                base_url,
                api_key,
                model,
                system_prompt,
                user_prompt,
            )
            .await
        }
        "anthropic_compatible" => {
            generate_anthropic_compatible(
                &client,
                base_url,
                api_key,
                model,
                system_prompt,
                user_prompt,
            )
            .await
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

    let parsed =
        Url::parse(trimmed).map_err(|_| AppError::new(E_VALIDATION, "Base URL 格式无效", false))?;

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

    handle_response(
        "openai_compatible",
        &endpoint,
        api_key,
        response,
        parse_openai_text,
    )
    .await
}

async fn generate_openai_compatible(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, AppError> {
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": 0.2,
            "stream": false,
        }))
        .send()
        .await
        .map_err(|error| map_request_error("openai_compatible", &endpoint, api_key, error))?;

    handle_response(
        "openai_compatible",
        &endpoint,
        api_key,
        response,
        parse_openai_text,
    )
    .await
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

async fn generate_anthropic_compatible(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, AppError> {
    let endpoint = format!("{}/messages", base_url.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": model,
            "system": system_prompt,
            "messages": [{ "role": "user", "content": user_prompt }],
            "max_tokens": 4096,
            "temperature": 0.2,
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

fn build_client(timeout_secs: u64) -> Result<Client, AppError> {
    build_client_with_timeout(std::time::Duration::from_secs(timeout_secs))
}

fn build_client_with_timeout(timeout: std::time::Duration) -> Result<Client, AppError> {
    Client::builder().timeout(timeout).build().map_err(|error| {
        AppError::new(
            E_PROVIDER_ERROR,
            format!("无法创建 HTTP 客户端: {error}"),
            false,
        )
    })
}

async fn handle_response(
    provider_type: &str,
    endpoint: &str,
    api_key: &str,
    response: reqwest::Response,
    parser: fn(&Value) -> Option<String>,
) -> Result<String, AppError> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let text = response.text().await.unwrap_or_default();
    let sanitized_text = redact::truncate(&redact::redact(&text, &[api_key]), 240);

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(
            AppError::new(E_AUTH_401, "Provider 鉴权失败，请检查 API Key", false).with_details(
                json!({
                    "providerType": provider_type,
                    "httpStatus": status.as_u16(),
                    "contentType": content_type,
                    "requestUrl": redact::sanitize_url(endpoint),
                    "bodyExcerpt": sanitized_text,
                    "sanitizedBodyExcerpt": sanitized_text,
                }),
            ),
        );
    }

    if !status.is_success() {
        return Err(
            AppError::new(E_PROVIDER_ERROR, "Provider 返回了非成功状态", true).with_details(
                json!({
                    "providerType": provider_type,
                    "httpStatus": status.as_u16(),
                    "contentType": content_type,
                    "requestUrl": redact::sanitize_url(endpoint),
                    "bodyExcerpt": sanitized_text,
                    "sanitizedBodyExcerpt": sanitized_text,
                }),
            ),
        );
    }

    if serde_json::from_str::<Value>(&text).is_err() {
        return Err(AppError::new(
            E_PROVIDER_RESPONSE_INVALID,
            "Provider returned a non-JSON HTTP response. Check Base URL, provider type, proxy error pages, or streaming/SSE settings.",
            false,
        )
        .with_details(json!({
            "providerType": provider_type,
            "httpStatus": status.as_u16(),
            "contentType": content_type,
            "requestUrl": redact::sanitize_url(endpoint),
            "bodyExcerpt": sanitized_text,
            "sanitizedBodyExcerpt": sanitized_text,
        })));
    }

    let payload: Value = serde_json::from_str(&text).map_err(|_| {
        AppError::new(
            E_PROVIDER_RESPONSE_INVALID,
            "Provider 响应不是有效 JSON",
            false,
        )
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
        .find_map(
            |block| match (block.get("type")?.as_str(), block.get("text")?.as_str()) {
                (Some("text"), Some(text)) => Some(text.to_string()),
                _ => None,
            },
        )
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

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::{sleep, Duration},
    };

    use super::*;
    use crate::types::error_codes::{E_AUTH_401, E_NET_TIMEOUT, E_PROVIDER_RESPONSE_INVALID};

    #[tokio::test]
    async fn openai_provider_successfully_extracts_text() {
        let base_url = spawn_single_response_server(
            http_json_response(200, r#"{"choices":[{"message":{"content":"OK"}}]}"#),
            Duration::from_millis(0),
        )
        .await;

        let content = test_provider("openai_compatible", &base_url, "sk-openai-secret", "demo")
            .await
            .unwrap();

        assert_eq!(content, "OK");
    }

    #[tokio::test]
    async fn anthropic_provider_successfully_extracts_text() {
        let base_url = spawn_single_response_server(
            http_json_response(200, r#"{"content":[{"type":"text","text":"OK"}]}"#),
            Duration::from_millis(0),
        )
        .await;

        let content = test_provider(
            "anthropic_compatible",
            &base_url,
            "sk-anthropic-secret",
            "demo",
        )
        .await
        .unwrap();

        assert_eq!(content, "OK");
    }

    #[tokio::test]
    async fn provider_maps_401_and_redacts_secret_from_details() {
        let base_url = spawn_single_response_server(
            http_json_response(401, r#"{"error":"invalid key sk-auth-secret"}"#),
            Duration::from_millis(0),
        )
        .await;

        let error = test_provider("openai_compatible", &base_url, "sk-auth-secret", "demo")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_AUTH_401);
        let details = error.details.unwrap().to_string();
        assert!(!details.contains("sk-auth-secret"));
        assert!(details.contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn provider_maps_timeout_errors() {
        let base_url = spawn_single_response_server(
            http_json_response(200, r#"{"choices":[{"message":{"content":"late"}}]}"#),
            Duration::from_millis(200),
        )
        .await;
        let client = build_client_with_timeout(Duration::from_millis(50)).unwrap();

        let error = test_openai_compatible(&client, &base_url, "sk-timeout-secret", "demo")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_NET_TIMEOUT);
    }

    #[tokio::test]
    async fn provider_maps_invalid_response_schema() {
        let base_url = spawn_single_response_server(
            http_json_response(200, r#"{}"#),
            Duration::from_millis(0),
        )
        .await;

        let error = test_provider("openai_compatible", &base_url, "sk-invalid-secret", "demo")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_PROVIDER_RESPONSE_INVALID);
    }

    #[tokio::test]
    async fn provider_maps_html_success_response_as_invalid_json_with_diagnostics() {
        let base_url = spawn_single_response_server(
            http_response(
                200,
                "text/html",
                "<html><body>proxy login sk-html-secret</body></html>",
            ),
            Duration::from_millis(0),
        )
        .await;

        let error = test_provider("openai_compatible", &base_url, "sk-html-secret", "demo")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_PROVIDER_RESPONSE_INVALID);
        assert!(error.message.contains("non-JSON HTTP response"));
        let details = error.details.unwrap().to_string();
        assert!(details.contains("text/html"));
        assert!(details.contains("bodyExcerpt"));
        assert!(!details.contains("sk-html-secret"));
        assert!(details.contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn provider_maps_plain_text_success_response_as_invalid_json() {
        let base_url = spawn_single_response_server(
            http_response(200, "text/plain", "not json"),
            Duration::from_millis(0),
        )
        .await;

        let error = test_provider("openai_compatible", &base_url, "sk-plain-secret", "demo")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_PROVIDER_RESPONSE_INVALID);
        let details = error.details.unwrap().to_string();
        assert!(details.contains("text/plain"));
        assert!(details.contains("not json"));
    }

    async fn spawn_single_response_server(response: String, delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = socket.read(&mut buffer).await;
            if !delay.is_zero() {
                sleep(delay).await;
            }
            let _ = socket.write_all(response.as_bytes()).await;
        });

        format!("http://127.0.0.1:{}", address.port())
    }

    fn http_json_response(status: u16, body: &str) -> String {
        http_response(status, "application/json", body)
    }

    fn http_response(status: u16, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status} TEST\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body,
        )
    }
}
