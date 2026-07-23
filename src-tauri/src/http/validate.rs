//! IPC payload validation. Input is untrusted: everything is checked here and
//! surfaced as typed `AppError`s before reqwest sees any of it — raw builder
//! errors must never reach the frontend.

use reqwest::header::{HeaderName, HeaderValue};

use crate::error::{AppError, ErrorKind};

use super::types::{SendBody, SendRequestPayload};

pub const MAX_ID_CHARS: usize = 128;
pub const MIN_TIMEOUT_MS: u64 = 1;
pub const MAX_TIMEOUT_MS: u64 = 600_000;
pub const MIN_MAX_BODY_BYTES: u64 = 1024 * 1024;
pub const MAX_MAX_BODY_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_TEXT_BODY_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_HEADER_ROWS: usize = 200;
pub const MAX_HEADER_AGGREGATE_BYTES: usize = 256 * 1024;

const ALLOWED_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

fn validation(message: impl Into<String>) -> AppError {
    AppError::new(ErrorKind::Validation, message)
}

fn invalid_url(message: impl Into<String>) -> AppError {
    AppError::new(ErrorKind::InvalidUrl, message)
}

fn check_id(value: &str, field: &str) -> Result<(), AppError> {
    if value.is_empty() {
        return Err(validation(format!("{field} must not be empty")));
    }
    if value.chars().count() > MAX_ID_CHARS {
        return Err(validation(format!(
            "{field} must be at most {MAX_ID_CHARS} characters"
        )));
    }
    Ok(())
}

pub fn validate_payload(payload: &SendRequestPayload) -> Result<(), AppError> {
    check_id(&payload.execution_id, "executionId")?;
    check_id(&payload.tab_id, "tabId")?;

    if !ALLOWED_METHODS.contains(&payload.method.as_str()) {
        return Err(validation(format!(
            "unsupported HTTP method \"{}\"",
            truncate_for_message(&payload.method)
        )));
    }

    let url = reqwest::Url::parse(&payload.url)
        .map_err(|_| invalid_url("the request URL is not a valid absolute URL"))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(invalid_url(format!(
            "unsupported URL scheme \"{}\" — only http and https are allowed",
            truncate_for_message(url.scheme())
        )));
    }

    if payload.headers.len() > MAX_HEADER_ROWS {
        return Err(validation(format!(
            "too many header rows (limit is {MAX_HEADER_ROWS})"
        )));
    }
    let mut header_bytes: usize = 0;
    for header in &payload.headers {
        // Never echo header values in errors — they may hold credentials.
        HeaderName::from_bytes(header.name.as_bytes()).map_err(|_| {
            validation(format!(
                "invalid header name \"{}\"",
                truncate_for_message(&header.name)
            ))
        })?;
        HeaderValue::from_str(&header.value).map_err(|_| {
            validation(format!(
                "header \"{}\" has an invalid value",
                truncate_for_message(&header.name)
            ))
        })?;
        header_bytes += header.name.len() + header.value.len();
    }
    if header_bytes > MAX_HEADER_AGGREGATE_BYTES {
        return Err(validation(format!(
            "headers exceed the aggregate size limit of {MAX_HEADER_AGGREGATE_BYTES} bytes"
        )));
    }

    if let Some(timeout_ms) = payload.timeout_ms {
        if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
            return Err(validation(format!(
                "timeout must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS} milliseconds"
            )));
        }
    }

    if !(MIN_MAX_BODY_BYTES..=MAX_MAX_BODY_BYTES).contains(&payload.max_body_bytes) {
        return Err(validation(format!(
            "maxBodyBytes must be between {MIN_MAX_BODY_BYTES} and {MAX_MAX_BODY_BYTES} bytes"
        )));
    }

    match &payload.body {
        SendBody::None => {}
        SendBody::Text { content } => {
            if content.len() > MAX_TEXT_BODY_BYTES {
                return Err(validation(format!(
                    "request body exceeds the {MAX_TEXT_BODY_BYTES} byte limit"
                )));
            }
        }
        SendBody::Multipart { .. } => {
            return Err(validation(
                "multipart bodies are not supported until M4".to_string(),
            ));
        }
        SendBody::File { .. } => {
            return Err(validation(
                "file bodies are not supported until M4".to_string(),
            ));
        }
    }

    Ok(())
}

/// Keep attacker-controlled strings short in error messages.
fn truncate_for_message(value: &str) -> String {
    const LIMIT: usize = 64;
    if value.chars().count() <= LIMIT {
        value.to_string()
    } else {
        let prefix: String = value.chars().take(LIMIT).collect();
        format!("{prefix}…")
    }
}
