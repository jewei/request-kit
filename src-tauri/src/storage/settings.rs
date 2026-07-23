//! `settings.json` read/write. Rust keeps settings as an opaque `Value`
//! envelope with a version check; the field schema lives in the TS `Settings`
//! type (same pattern as `RequestFile`). A missing file yields defaults; a
//! corrupt/too-new file is quarantined and defaults are returned so a bad
//! settings file never blocks startup.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::AppError;
use crate::storage::atomic::write_json_atomic;
use crate::storage::quarantine::quarantine;

pub fn settings_path(root: &Path) -> PathBuf {
    root.join("settings.json")
}

/// The built-in defaults, also returned when no settings file exists yet.
pub fn defaults() -> Value {
    json!({
        "version": 1,
        "theme": "system",
        "fontSize": 13,
        "timeoutMs": 30000,
        "followRedirects": true,
        "maxBodyBytes": 10_485_760,
        "editorLargeFileKb": 1024
    })
}

pub fn read_settings(root: &Path) -> Value {
    let path = settings_path(root);
    if !path.exists() {
        return defaults();
    }
    let parsed: Option<Value> = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    match parsed {
        Some(value) if value.get("version").and_then(Value::as_u64) == Some(1) => value,
        Some(_) => {
            quarantine(&path, "settings.json has an unsupported version");
            defaults()
        }
        None => {
            quarantine(&path, "settings.json is corrupt");
            defaults()
        }
    }
}

pub fn write_settings(root: &Path, value: &Value) -> Result<(), AppError> {
    write_json_atomic(&settings_path(root), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_settings(dir.path()), defaults());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut custom = defaults();
        custom["theme"] = json!("dark");
        write_settings(dir.path(), &custom).unwrap();
        assert_eq!(read_settings(dir.path())["theme"], "dark");
    }

    #[test]
    fn corrupt_file_quarantines_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(settings_path(dir.path()), b"{ not json").unwrap();

        assert_eq!(read_settings(dir.path()), defaults());
        let quarantined = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(quarantined);
    }

    #[test]
    fn too_new_version_quarantines_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            settings_path(dir.path()),
            serde_json::to_vec(&json!({ "version": 2, "theme": "dark" })).unwrap(),
        )
        .unwrap();
        assert_eq!(read_settings(dir.path()), defaults());
    }
}
