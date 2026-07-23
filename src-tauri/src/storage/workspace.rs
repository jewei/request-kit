//! The stateful workspace: the node tree + `id → path` index built from disk,
//! held in `AppState`. Disk is the single source of truth; this is a
//! rebuildable cache. Each mutation writes to disk then rebuilds the index in
//! the same locked section (design spec, Approach 1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{AppError, ErrorKind};
use crate::storage::nodes::WorkspaceNode;
use crate::storage::paths::collections_dir;
use crate::storage::quarantine::{quarantine, QuarantineReport};
use crate::storage::scan::scan;

pub struct Workspace {
    root: PathBuf,
    tree: Vec<WorkspaceNode>,
    index: HashMap<String, PathBuf>,
}

impl Workspace {
    /// Scans `root/collections` and returns the workspace plus any files that
    /// were quarantined during the scan.
    pub fn load(root: PathBuf) -> (Self, Vec<QuarantineReport>) {
        let result = scan(&collections_dir(&root));
        (
            Self {
                root,
                tree: result.tree,
                index: result.index,
            },
            result.quarantined,
        )
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn tree(&self) -> Vec<WorkspaceNode> {
        self.tree.clone()
    }

    pub fn path_for(&self, id: &str) -> Option<&PathBuf> {
        self.index.get(id)
    }

    /// Re-scan disk and replace the in-memory tree + index. Called after every
    /// mutation so ordering and quarantine logic stay single-sourced in `scan`.
    pub(crate) fn refresh(&mut self) -> Vec<QuarantineReport> {
        let result = scan(&collections_dir(&self.root));
        self.tree = result.tree;
        self.index = result.index;
        result.quarantined
    }

    /// Reads a request document by id. Errors if the id is unknown or the file
    /// is not a request; a corrupt file is quarantined and reported as an error.
    pub fn read_request(&self, id: &str) -> Result<Value, AppError> {
        let path = self
            .index
            .get(id)
            .ok_or_else(|| AppError::new(ErrorKind::Validation, format!("unknown id: {id}")))?;
        if path.is_dir() {
            return Err(AppError::new(
                ErrorKind::Validation,
                format!("id {id} is a collection or folder, not a request"),
            ));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| AppError::io(format!("could not read request: {e}")))?;
        serde_json::from_slice(&bytes).map_err(|e| {
            quarantine(path, format!("invalid JSON: {e}"));
            AppError::new(ErrorKind::Io, "the request file was corrupt and has been quarantined")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::atomic::write_json_atomic;
    use crate::storage::nodes::{collection_meta, default_request_file};
    use crate::storage::paths::collections_dir;

    fn write(path: &Path, value: &Value) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_json_atomic(path, value).unwrap();
    }

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let col = collections_dir(root.path()).join("col");
        write(&col.join("collection.json"), &collection_meta("c1", "Col"));
        write(&col.join("req.json"), &default_request_file("r1", "Req"));
        root
    }

    #[test]
    fn load_builds_tree_and_index() {
        let root = fixture();
        let (ws, quarantined) = Workspace::load(root.path().to_path_buf());
        assert!(quarantined.is_empty());
        assert_eq!(ws.tree().len(), 1);
        assert!(ws.path_for("r1").is_some());
    }

    #[test]
    fn read_request_returns_document() {
        let root = fixture();
        let (ws, _) = Workspace::load(root.path().to_path_buf());
        let doc = ws.read_request("r1").unwrap();
        assert_eq!(doc["id"], "r1");
        assert_eq!(doc["method"], "GET");
    }

    #[test]
    fn read_unknown_id_errors() {
        let root = fixture();
        let (ws, _) = Workspace::load(root.path().to_path_buf());
        let err = ws.read_request("nope").unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
    }

    #[test]
    fn read_container_id_errors() {
        let root = fixture();
        let (ws, _) = Workspace::load(root.path().to_path_buf());
        let err = ws.read_request("c1").unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
    }
}
