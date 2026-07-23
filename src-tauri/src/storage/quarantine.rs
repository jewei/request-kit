//! Corrupt / duplicate-UUID files are renamed aside rather than deleted or
//! ignored, so a bad file never blocks startup and the user can recover it
//! (PLAN.md, storage design).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// A file that was moved aside during a workspace scan.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineReport {
    /// The path the file had before it was quarantined.
    pub original: String,
    pub reason: String,
}

/// Renames `path` to `<path>.corrupt-<unixSecs>` (best effort) and returns a
/// report. The report is returned even if the rename fails, so the caller can
/// still surface the problem.
pub fn quarantine(path: &Path, reason: impl Into<String>) -> QuarantineReport {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut aside = path.as_os_str().to_owned();
    aside.push(format!(".corrupt-{ts}"));
    let _ = std::fs::rename(path, &aside);

    QuarantineReport {
        original: path.display().to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_file_aside_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("broken.json");
        std::fs::write(&bad, b"{ not json").unwrap();

        let report = quarantine(&bad, "invalid JSON");

        assert!(!bad.exists(), "original path should be gone");
        assert_eq!(report.reason, "invalid JSON");
        assert!(report.original.ends_with("broken.json"));

        // The aside file exists with the .corrupt- infix.
        let survivor = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(survivor.is_some());
    }
}
