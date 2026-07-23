//! Atomic, durable JSON writes. All storage replacement goes through this one
//! helper so temp-file placement, fsync, and permissions stay consistent
//! (PLAN.md, "Atomic writes").

use std::path::Path;

use serde_json::Value;
use tempfile::NamedTempFile;

use crate::error::AppError;

/// Serializes `value` to `target` atomically and durably:
/// temp file in the same dir → flush → fsync → rename into place. The original
/// file is preserved if any step fails. Refuses to write through a symlink.
pub fn write_json_atomic(target: &Path, value: &Value) -> Result<(), AppError> {
    // Never follow a symlink into storage (PLAN.md security boundary).
    if let Ok(meta) = std::fs::symlink_metadata(target) {
        if meta.file_type().is_symlink() {
            return Err(AppError::io("refusing to write through a symlink"));
        }
    }

    let parent = target
        .parent()
        .ok_or_else(|| AppError::io("target path has no parent directory"))?;

    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|e| AppError::io(format!("could not create temp file: {e}")))?;

    serde_json::to_writer_pretty(temp.as_file_mut(), value)
        .map_err(|e| AppError::io(format!("could not serialize JSON: {e}")))?;

    {
        use std::io::Write as _;
        temp.as_file_mut()
            .flush()
            .map_err(|e| AppError::io(format!("could not flush temp file: {e}")))?;
    }
    // flush() alone is not durable — the bytes must reach the disk.
    temp.as_file()
        .sync_all()
        .map_err(|e| AppError::io(format!("could not fsync temp file: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o600))
            .map_err(|e| AppError::io(format!("could not set file permissions: {e}")))?;
    }

    temp.persist(target)
        .map_err(|e| AppError::io(format!("could not persist file: {e}")))?;

    // Best-effort parent-dir fsync so the rename itself is durable (Unix only).
    #[cfg(unix)]
    {
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.json");
        write_json_atomic(&target, &json!({ "a": 1 })).unwrap();

        let back: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(back, json!({ "a": 1 }));
        // No temp residue: exactly the one file we wrote.
        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.json");
        write_json_atomic(&target, &json!({ "v": 1 })).unwrap();
        write_json_atomic(&target, &json!({ "v": 2 })).unwrap();

        let back: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(back, json!({ "v": 2 }));
        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_symlink_target() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.json");
        std::fs::write(&real, b"{}").unwrap();
        let link = dir.path().join("link.json");
        symlink(&real, &link).unwrap();

        let err = write_json_atomic(&link, &json!({ "x": 1 })).unwrap_err();
        assert!(err.message.contains("symlink"));
    }

    #[test]
    #[cfg(unix)]
    fn written_file_is_mode_600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.json");
        write_json_atomic(&target, &json!({})).unwrap();
        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
