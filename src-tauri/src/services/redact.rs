use std::sync::OnceLock;

use regex::{Captures, Regex};

const REDACTED: &str = "[REDACTED]";

pub fn redact(text: &str, secrets: &[&str]) -> String {
    let mut sanitized = text.to_string();

    for secret in secrets {
        if !secret.is_empty() {
            sanitized = sanitized.replace(secret, REDACTED);
        }
    }

    sanitized = authorization_regex().replace_all(&sanitized, "$1[REDACTED]").into_owned();
    sanitized = secret_field_regex()
        .replace_all(&sanitized, |captures: &Captures<'_>| format!("{}[REDACTED]", &captures[1]))
        .into_owned();
    query_regex()
        .replace_all(&sanitized, |captures: &Captures<'_>| format!("{}[REDACTED]", &captures[1]))
        .into_owned()
}

pub fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    let truncated: String = text.chars().take(limit).collect();
    format!("{truncated}…")
}

pub fn sanitize_url(url: &str) -> String {
    redact(url, &[])
}

fn authorization_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(authorization\s*:\s*bearer\s+)[^\s\"',}] +"#.replace(" ", "").as_str())
            .expect("valid authorization regex")
    })
}

fn secret_field_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)((?:\"(?:api_key|x-api-key|secret|authorization)\"|(?:api_key|x-api-key|secret|authorization))\s*[:=]\s*\"?)[^\"\s,}] +"#.replace(" ", "").as_str())
            .expect("valid secret field regex")
    })
}

fn query_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)((?:api_key|x-api-key|secret|authorization)=)[^&\s]+"#)
            .expect("valid query regex")
    })
}