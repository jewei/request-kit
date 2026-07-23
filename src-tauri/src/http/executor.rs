//! The send flow: validate → maintenance-mode gate → register cancellation →
//! execute with streaming download cap → classify text/binary → retain full
//! bytes → build the (possibly truncated) IPC response. Failed or cancelled
//! sends retain nothing.

use std::error::Error;
use std::sync::PoisonError;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, ErrorKind};
use crate::state::{AppMode, AppState};

use super::error_map;
use super::types::{HttpResponseData, SendBody, SendRequestPayload};
use super::validate;

/// IPC display cap: at most 2 MB of UTF-8 text crosses the boundary.
pub const IPC_BODY_DISPLAY_CAP: usize = 2 * 1024 * 1024;

/// Removes the abort-map entry when the send future completes or is dropped.
struct AbortGuard<'a> {
    state: &'a AppState,
    execution_id: String,
}

impl Drop for AbortGuard<'_> {
    fn drop(&mut self) {
        self.state
            .abort_map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.execution_id);
    }
}

pub async fn send(
    state: &AppState,
    payload: SendRequestPayload,
) -> Result<HttpResponseData, AppError> {
    validate::validate_payload(&payload)?;

    {
        let mode = state.mode.lock().unwrap_or_else(PoisonError::into_inner);
        if *mode != AppMode::Normal {
            return Err(AppError::new(
                ErrorKind::MaintenanceInProgress,
                "a maintenance operation is in progress — try again once it completes",
            ));
        }
    }

    let token = CancellationToken::new();
    state
        .abort_map
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(payload.execution_id.clone(), token.clone());
    let _guard = AbortGuard {
        state,
        execution_id: payload.execution_id.clone(),
    };

    tokio::select! {
        _ = token.cancelled() => Err(AppError::new(ErrorKind::Cancelled, "request cancelled")),
        result = perform(state, &payload) => result,
    }
}

async fn perform(
    state: &AppState,
    payload: &SendRequestPayload,
) -> Result<HttpResponseData, AppError> {
    let client = if payload.follow_redirects {
        &state.http_clients.follow_redirects
    } else {
        &state.http_clients.no_redirects
    };

    // Validation already vetted all of these; failures here are defensive.
    let method = reqwest::Method::from_bytes(payload.method.as_bytes())
        .map_err(|_| AppError::new(ErrorKind::Validation, "unsupported HTTP method"))?;
    let mut headers = HeaderMap::new();
    for header in &payload.headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| AppError::new(ErrorKind::Validation, "invalid header name"))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|_| AppError::new(ErrorKind::Validation, "invalid header value"))?;
        headers.append(name, value);
    }

    let mut builder = client.request(method, &payload.url).headers(headers);
    if let Some(timeout_ms) = payload.timeout_ms {
        builder = builder.timeout(Duration::from_millis(timeout_ms));
    }
    match &payload.body {
        SendBody::None => {}
        SendBody::Text { content } => builder = builder.body(content.clone()),
        SendBody::Multipart { .. } | SendBody::File { .. } => {
            return Err(AppError::new(
                ErrorKind::Validation,
                "multipart and file bodies are not supported until M4",
            ));
        }
    }

    let started = Instant::now();
    let mut response = builder.send().await.map_err(|e| transport_error(&e))?;

    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("").to_string();
    let http_version = version_label(response.version());
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();

    // Stream the (decoded) body, hard-capping at max_body_bytes.
    let cap = payload.max_body_bytes as usize;
    let mut bytes: Vec<u8> = Vec::new();
    let mut download_capped = false;
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                let remaining = cap.saturating_sub(bytes.len());
                if chunk.len() > remaining {
                    bytes.extend_from_slice(&chunk[..remaining]);
                    download_capped = true;
                    break;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => return Err(transport_error(&e)),
        }
    }
    // Application-observed duration: just before execute → last body chunk.
    let duration_ms = started.elapsed().as_millis() as u64;
    let body_bytes = bytes.len() as u64;

    let text = classify_body(content_type.as_deref(), &bytes);
    let is_binary = text.is_none();
    let (body, body_truncated) = match text {
        Some(t) if t.len() > IPC_BODY_DISPLAY_CAP => (
            Some(floor_char_boundary(t, IPC_BODY_DISPLAY_CAP).to_string()),
            true,
        ),
        Some(t) => (Some(t.to_string()), false),
        None => (None, false),
    };

    // Retain the FULL downloaded bytes for save-to-file. Only successful sends
    // reach this point — failures and cancellations retain nothing.
    state.retention.insert(&payload.execution_id, bytes);

    Ok(HttpResponseData {
        execution_id: payload.execution_id.clone(),
        status: status.as_u16(),
        status_text,
        http_version,
        headers: response_headers,
        duration_ms,
        body_bytes,
        content_type,
        final_url,
        body,
        body_truncated,
        is_binary,
        download_capped,
    })
}

/// Deterministic text/binary classification (PLAN.md). Returns the body as
/// `&str` when it should be treated as text, `None` when binary.
pub fn classify_body<'a>(content_type: Option<&str>, bytes: &'a [u8]) -> Option<&'a str> {
    let essence = content_type.map(|ct| {
        ct.split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
    });
    match essence.as_deref() {
        // Clearly binary types are binary even when the bytes are valid UTF-8.
        Some(e) if is_binary_type(e) => None,
        // Textual types: attempt UTF-8; invalid ⇒ binary.
        Some(e) if is_textual_type(e) => std::str::from_utf8(bytes).ok(),
        // Missing/unknown ⇒ text iff valid UTF-8 with no NUL bytes.
        _ => match std::str::from_utf8(bytes) {
            Ok(text) if !bytes.contains(&0) => Some(text),
            _ => None,
        },
    }
}

fn is_textual_type(essence: &str) -> bool {
    essence.starts_with("text/")
        || essence == "application/json"
        || essence == "application/xml"
        || essence == "application/javascript"
        || essence == "application/graphql"
        || (essence.starts_with("application/")
            && (essence.ends_with("+json") || essence.ends_with("+xml")))
}

fn is_binary_type(essence: &str) -> bool {
    essence.starts_with("image/")
        || essence.starts_with("audio/")
        || essence.starts_with("video/")
        || essence.starts_with("font/")
        || essence == "application/octet-stream"
        || essence == "application/pdf"
        || essence == "application/zip"
}

fn transport_error(err: &(dyn Error + 'static)) -> AppError {
    let kind = error_map::classify(err);
    // The unredacted chain exists only in memory; detail is always redacted.
    let detail = error_map::redact_url_like(&error_map::chain_text(err));
    AppError {
        kind,
        message: friendly_message(kind).to_string(),
        detail: Some(detail),
    }
}

fn friendly_message(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidUrl => "the request URL is invalid",
        ErrorKind::Cancelled => "request cancelled",
        ErrorKind::Timeout => "the request timed out",
        ErrorKind::Tls => "could not establish a secure (TLS) connection",
        ErrorKind::Dns => "could not resolve the host name",
        ErrorKind::Connection => "could not connect to the server",
        ErrorKind::Io => "an I/O error occurred while sending the request",
        ErrorKind::Validation => "the request payload is invalid",
        ErrorKind::MaintenanceInProgress => "a maintenance operation is in progress",
        ErrorKind::Unknown => "the request failed",
    }
}

fn version_label(version: reqwest::Version) -> String {
    use reqwest::Version;
    if version == Version::HTTP_09 {
        "HTTP/0.9"
    } else if version == Version::HTTP_10 {
        "HTTP/1.0"
    } else if version == Version::HTTP_11 {
        "HTTP/1.1"
    } else if version == Version::HTTP_2 {
        "HTTP/2"
    } else if version == Version::HTTP_3 {
        "HTTP/3"
    } else {
        "HTTP/?"
    }
    .to_string()
}

/// Largest prefix of `text` that is at most `max` bytes and ends on a char
/// boundary.
fn floor_char_boundary(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
