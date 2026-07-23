//! Append-only request history (`history/history.jsonl`). One compact JSON
//! object per line — appended (never atomic-replaced) so a crash can at most
//! tear the last line, which readers tolerate. Compacted to the newest
//! `HISTORY_COMPACTION_THRESHOLD` entries at read time.
//!
//! Redaction is the frontend's job (template URLs only); this module only
//! persists what it is given after a version check.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{AppError, ErrorKind};
use crate::storage::paths::ensure_dir;

pub const HISTORY_COMPACTION_THRESHOLD: usize = 500;

pub fn history_dir(root: &Path) -> PathBuf {
    root.join("history")
}

pub fn history_file(root: &Path) -> PathBuf {
    history_dir(root).join("history.jsonl")
}

/// Appends one entry as a JSON line. Rejects a missing/unsupported version.
pub fn append_history(root: &Path, entry: &Value) -> Result<(), AppError> {
    if entry.get("version").and_then(Value::as_u64) != Some(1) {
        return Err(AppError::new(
            ErrorKind::Validation,
            "history entry has a missing or unsupported version",
        ));
    }
    let dir = history_dir(root);
    ensure_dir(&dir)?;
    let file_path = history_file(root);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|e| AppError::io(format!("could not open history: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o600));
    }

    let line = serde_json::to_string(entry)
        .map_err(|e| AppError::io(format!("could not serialize history entry: {e}")))?;
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| AppError::io(format!("could not write history: {e}")))?;
    Ok(())
}

/// Returns up to `limit` entries, newest-first. Compacts the file to the newest
/// `HISTORY_COMPACTION_THRESHOLD` entries when it has grown past that.
pub fn read_history(root: &Path, limit: usize) -> Vec<Value> {
    let file_path = history_file(root);
    let Ok(text) = std::fs::read_to_string(&file_path) else {
        return Vec::new();
    };

    // File order = oldest-first; skip blank/torn/invalid lines.
    let mut entries: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();

    if entries.len() > HISTORY_COMPACTION_THRESHOLD {
        entries = entries.split_off(entries.len() - HISTORY_COMPACTION_THRESHOLD);
        let _ = rewrite(&file_path, &entries);
    }

    entries.reverse(); // newest-first
    entries.truncate(limit);
    entries
}

pub fn clear_history(root: &Path) -> Result<(), AppError> {
    let file_path = history_file(root);
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| AppError::io(format!("could not clear history: {e}")))?;
    }
    Ok(())
}

/// Atomically rewrite the log with the given oldest-first entries.
fn rewrite(file_path: &Path, entries: &[Value]) -> Result<(), AppError> {
    let parent = file_path
        .parent()
        .ok_or_else(|| AppError::io("history file has no parent"))?;
    let mut body = String::new();
    for entry in entries {
        if let Ok(line) = serde_json::to_string(entry) {
            body.push_str(&line);
            body.push('\n');
        }
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| AppError::io(format!("could not create temp file: {e}")))?;
    temp.write_all(body.as_bytes())
        .map_err(|e| AppError::io(format!("could not write compacted history: {e}")))?;
    temp.as_file().sync_all().ok();
    temp.persist(file_path)
        .map_err(|e| AppError::io(format!("could not persist compacted history: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(id: &str) -> Value {
        json!({ "version": 1, "id": id, "method": "GET", "templateUrl": "https://x/" })
    }

    #[test]
    fn append_then_read_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        append_history(dir.path(), &entry("a")).unwrap();
        append_history(dir.path(), &entry("b")).unwrap();
        append_history(dir.path(), &entry("c")).unwrap();

        let got = read_history(dir.path(), 10);
        let ids: Vec<&str> = got.iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["c", "b", "a"]);
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_history(dir.path(), 10).is_empty());
    }

    #[test]
    fn tolerates_blank_and_torn_lines() {
        let dir = tempfile::tempdir().unwrap();
        ensure_dir(&history_dir(dir.path())).unwrap();
        let file = history_file(dir.path());
        let content = format!(
            "{}\n\n{{ torn line\n{}\n",
            serde_json::to_string(&entry("a")).unwrap(),
            serde_json::to_string(&entry("b")).unwrap()
        );
        std::fs::write(&file, content).unwrap();

        let got = read_history(dir.path(), 10);
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn compacts_past_threshold() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..600 {
            append_history(dir.path(), &entry(&format!("e{i}"))).unwrap();
        }
        // Read triggers compaction.
        let got = read_history(dir.path(), 1000);
        assert_eq!(got.len(), HISTORY_COMPACTION_THRESHOLD);
        // Newest survived; oldest were dropped.
        assert_eq!(got[0]["id"], "e599");

        // On-disk file now holds exactly 500 lines.
        let lines = std::fs::read_to_string(history_file(dir.path())).unwrap();
        assert_eq!(lines.lines().filter(|l| !l.is_empty()).count(), 500);
    }

    #[test]
    fn clear_empties() {
        let dir = tempfile::tempdir().unwrap();
        append_history(dir.path(), &entry("a")).unwrap();
        clear_history(dir.path()).unwrap();
        assert!(read_history(dir.path(), 10).is_empty());
    }

    #[test]
    fn rejects_bad_version() {
        let dir = tempfile::tempdir().unwrap();
        let err = append_history(dir.path(), &json!({ "version": 2, "id": "x" })).unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
    }
}
