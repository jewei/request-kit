use serde::Serialize;

/// Typed error crossing the IPC boundary. `detail` must already be redacted
/// (no URL query values, no credentials) before an `AppError` is constructed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[allow(dead_code)] // full taxonomy is the IPC contract; variants constructed from M1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    InvalidUrl,
    Cancelled,
    Timeout,
    Tls,
    Dns,
    Connection,
    Io,
    Validation,
    MaintenanceInProgress,
    Unknown,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_kind_as_camel_case() {
        let err = AppError::new(ErrorKind::MaintenanceInProgress, "busy");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "maintenanceInProgress");
        assert_eq!(json["message"], "busy");
        assert!(json.get("detail").is_none());
    }
}
