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

    sanitized = authorization_regex()
        .replace_all(&sanitized, "$1[REDACTED]")
        .into_owned();
    sanitized = secret_field_regex()
        .replace_all(&sanitized, |captures: &Captures<'_>| {
            format!("{}[REDACTED]", &captures[1])
        })
        .into_owned();
    query_regex()
        .replace_all(&sanitized, |captures: &Captures<'_>| {
            format!("{}[REDACTED]", &captures[1])
        })
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
        Regex::new(
            r#"(?i)(authorization\s*:\s*bearer\s+)[^\s\"',}] +"#
                .replace(" ", "")
                .as_str(),
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_explicit_secret_authorization_and_query_values() {
        let secret = "sk-live-secret";
        let text = format!(
            "Authorization: Bearer {secret}; url=https://example.com?api_key={secret}&x-api-key={secret}; raw={secret}",
        );

        let sanitized = redact(&text, &[secret]);

        assert!(!sanitized.contains(secret));
        assert!(sanitized.contains("Authorization"));
        assert!(sanitized.contains("[REDACTED]"));
        assert!(sanitized.contains("api_key=[REDACTED]"));
        assert!(sanitized.matches("[REDACTED]").count() >= 3);
    }

    #[test]
    fn redacts_secret_like_fields() {
        let text = r#"{"api_key":"sk-test","secret":"abc","authorization":"Bearer raw","x-api-key":"xyz"}"#;

        let sanitized = redact(text, &[]);

        assert!(!sanitized.contains("sk-test"));
        assert!(!sanitized.contains("abc"));
        assert!(!sanitized.contains("Bearer raw"));
        assert!(!sanitized.contains("xyz"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
