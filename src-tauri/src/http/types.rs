//! Serde mirrors of the TS IPC types in `src/types/{request,response}.ts`.
//! All field names cross the boundary as camelCase.

use serde::{Deserialize, Serialize};

/// One request header row. Duplicate names are allowed and preserved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

/// Body sent over IPC — everything textual arrives pre-serialized by
/// `prepareRequest`. `multipart`/`file` are part of the wire contract but are
/// rejected by validation until M4.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum SendBody {
    None,
    Text { content: String },
    Multipart { parts: Vec<MultipartPart> },
    File { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MultipartPartKind {
    Text,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipartPart {
    pub name: String,
    pub kind: MultipartPartKind,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
}

/// Payload for the `send_request` command. IPC input is untrusted — everything
/// here is re-validated in `validate.rs` before reqwest sees any of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequestPayload {
    pub execution_id: String,
    pub tab_id: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub body: SendBody,
    pub timeout_ms: Option<u64>,
    pub follow_redirects: bool,
    pub max_body_bytes: u64,
}

/// Successful send result crossing IPC (mirrors `HttpResponseData` in TS).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseData {
    pub execution_id: String,
    pub status: u16,
    /// Canonical label for the status code, not the server's reason phrase.
    pub status_text: String,
    pub http_version: String,
    /// Duplicate values preserved; global ordering unspecified (HeaderMap).
    pub headers: Vec<(String, String)>,
    pub duration_ms: u64,
    /// Decoded body bytes received (after automatic content decoding).
    pub body_bytes: u64,
    pub content_type: Option<String>,
    pub final_url: String,
    /// UTF-8 text capped at the IPC display limit; `None` when binary.
    pub body: Option<String>,
    pub body_truncated: bool,
    pub is_binary: bool,
    pub download_capped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_body_deserializes_from_camel_case_tag() {
        let body: SendBody = serde_json::from_str(r#"{"mode":"text","content":"hi"}"#).unwrap();
        match body {
            SendBody::Text { content } => assert_eq!(content, "hi"),
            other => panic!("unexpected body: {other:?}"),
        }
        let none: SendBody = serde_json::from_str(r#"{"mode":"none"}"#).unwrap();
        assert!(matches!(none, SendBody::None));
    }

    #[test]
    fn payload_deserializes_from_camel_case_fields() {
        let payload: SendRequestPayload = serde_json::from_str(
            r#"{
                "executionId": "e1", "tabId": "t1", "method": "GET",
                "url": "https://example.com", "headers": [{"name":"A","value":"b"}],
                "body": {"mode":"none"}, "timeoutMs": null,
                "followRedirects": true, "maxBodyBytes": 1048576
            }"#,
        )
        .unwrap();
        assert_eq!(payload.execution_id, "e1");
        assert!(payload.timeout_ms.is_none());
        assert_eq!(payload.headers[0].name, "A");
    }

    #[test]
    fn response_serializes_camel_case() {
        let data = HttpResponseData {
            execution_id: "e1".into(),
            status: 200,
            status_text: "OK".into(),
            http_version: "HTTP/1.1".into(),
            headers: vec![],
            duration_ms: 1,
            body_bytes: 0,
            content_type: None,
            final_url: "https://example.com/".into(),
            body: None,
            body_truncated: false,
            is_binary: false,
            download_capped: false,
        };
        let json = serde_json::to_value(&data).unwrap();
        assert!(json.get("executionId").is_some());
        assert!(json.get("bodyTruncated").is_some());
        assert!(json.get("downloadCapped").is_some());
    }
}
