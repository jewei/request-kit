//! The stateful workspace: the node tree + `id → path` index built from disk,
//! held in `AppState`. Disk is the single source of truth; this is a
//! rebuildable cache. Each mutation writes to disk then rebuilds the index in
//! the same locked section (design spec, Approach 1).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, ErrorKind};
use crate::storage::atomic::write_json_atomic;
use crate::storage::nodes::{
    collection_meta, default_request_file, folder_meta, WorkspaceNode,
};
use crate::storage::paths::{collections_dir, ensure_dir};
use crate::storage::quarantine::{quarantine, QuarantineReport};
use crate::storage::scan::{scan, COLLECTION_META, FOLDER_META};
use crate::storage::slug::{slugify, unique_slug};

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

/// Mutations. Each writes to disk, then rebuilds the in-memory tree + index via
/// `refresh`, then returns the affected node (from the fresh tree).
impl Workspace {
    pub fn create_collection(&mut self, name: &str) -> Result<WorkspaceNode, AppError> {
        let base = collections_dir(&self.root);
        ensure_dir(&base)?;
        let slug = unique_slug(&slugify(name), &|s| base.join(s).exists());
        let dir = base.join(&slug);
        ensure_dir(&dir)?;
        let id = Uuid::new_v4().to_string();
        write_json_atomic(&dir.join(COLLECTION_META), &collection_meta(&id, name))?;
        self.refresh();
        self.node_or_err(&id)
    }

    pub fn create_folder(&mut self, parent_id: &str, name: &str) -> Result<WorkspaceNode, AppError> {
        let parent = self.container_dir(parent_id)?;
        let slug = unique_slug(&slugify(name), &|s| parent.join(s).exists());
        let dir = parent.join(&slug);
        ensure_dir(&dir)?;
        let id = Uuid::new_v4().to_string();
        write_json_atomic(&dir.join(FOLDER_META), &folder_meta(&id, name))?;
        self.refresh();
        self.node_or_err(&id)
    }

    pub fn create_request(&mut self, parent_id: &str, name: &str) -> Result<WorkspaceNode, AppError> {
        let parent = self.container_dir(parent_id)?;
        let slug = unique_slug(&slugify(name), &|s| parent.join(format!("{s}.json")).exists());
        let path = parent.join(format!("{slug}.json"));
        let id = Uuid::new_v4().to_string();
        write_json_atomic(&path, &default_request_file(&id, name))?;
        self.refresh();
        self.node_or_err(&id)
    }

    /// Overwrites a request's document. The id and version are immutable; name
    /// changes to the filename go through `rename_node`, not here.
    pub fn write_request(&mut self, id: &str, document: Value) -> Result<(), AppError> {
        let path = self.request_path(id)?;
        if document.get("version").and_then(Value::as_u64) != Some(1) {
            return Err(AppError::new(
                ErrorKind::Validation,
                "request document has a missing or unsupported version",
            ));
        }
        if document.get("id").and_then(Value::as_str) != Some(id) {
            return Err(AppError::new(
                ErrorKind::Validation,
                "request document id does not match the target id",
            ));
        }
        write_json_atomic(&path, &document)
    }

    pub fn rename_node(&mut self, id: &str, new_name: &str) -> Result<WorkspaceNode, AppError> {
        let path = self.resolve(id)?;
        if path.is_dir() {
            self.rename_container(&path, new_name)?;
        } else {
            rename_request(&path, new_name)?;
        }
        self.refresh();
        self.node_or_err(id)
    }

    pub fn delete_node(&mut self, id: &str) -> Result<(), AppError> {
        let path = self.resolve(id)?;
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|e| AppError::io(format!("could not delete: {e}")))?;
        } else {
            fs::remove_file(&path).map_err(|e| AppError::io(format!("could not delete: {e}")))?;
        }
        self.refresh();
        Ok(())
    }

    pub fn duplicate_request(&mut self, id: &str) -> Result<WorkspaceNode, AppError> {
        let path = self.request_path(id)?;
        let parent = path.parent().ok_or_else(|| AppError::io("no parent dir"))?;
        let mut doc = read_doc(&path)?;
        let new_id = Uuid::new_v4().to_string();
        let new_name = format!("{} copy", doc["name"].as_str().unwrap_or("request"));
        doc["id"] = json!(new_id);
        doc["name"] = json!(new_name);
        let slug = unique_slug(&slugify(&new_name), &|s| {
            parent.join(format!("{s}.json")).exists()
        });
        write_json_atomic(&parent.join(format!("{slug}.json")), &doc)?;
        self.refresh();
        self.node_or_err(&new_id)
    }

    // --- helpers ---

    fn resolve(&self, id: &str) -> Result<PathBuf, AppError> {
        self.index
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorKind::Validation, format!("unknown id: {id}")))
    }

    fn container_dir(&self, id: &str) -> Result<PathBuf, AppError> {
        let path = self.resolve(id)?;
        if !path.is_dir() {
            return Err(AppError::new(
                ErrorKind::Validation,
                format!("id {id} is not a collection or folder"),
            ));
        }
        Ok(path)
    }

    fn request_path(&self, id: &str) -> Result<PathBuf, AppError> {
        let path = self.resolve(id)?;
        if path.is_dir() {
            return Err(AppError::new(
                ErrorKind::Validation,
                format!("id {id} is not a request"),
            ));
        }
        Ok(path)
    }

    fn rename_container(&self, old_dir: &Path, new_name: &str) -> Result<(), AppError> {
        let parent = old_dir.parent().ok_or_else(|| AppError::io("no parent dir"))?;
        let is_collection = parent == collections_dir(&self.root);
        let meta_name = if is_collection { COLLECTION_META } else { FOLDER_META };
        let meta_path = old_dir.join(meta_name);
        let mut meta = read_doc(&meta_path)?;
        meta["name"] = json!(new_name);
        write_json_atomic(&meta_path, &meta)?; // update name in place first

        let old_name = file_name(old_dir);
        let slug = unique_slug(&slugify(new_name), &|s| {
            !s.eq_ignore_ascii_case(&old_name) && parent.join(s).exists()
        });
        let new_dir = parent.join(&slug);
        if new_dir != old_dir {
            // Two-step handles a case-only rename on a case-insensitive FS.
            let temp = parent.join(format!(".rk-tmp-{}", Uuid::new_v4()));
            fs::rename(old_dir, &temp).map_err(|e| AppError::io(format!("rename failed: {e}")))?;
            fs::rename(&temp, &new_dir).map_err(|e| AppError::io(format!("rename failed: {e}")))?;
        }
        Ok(())
    }

    fn node_or_err(&self, id: &str) -> Result<WorkspaceNode, AppError> {
        find_node(&self.tree, id)
            .ok_or_else(|| AppError::io("node not found after write (internal error)"))
    }
}

fn rename_request(old_path: &Path, new_name: &str) -> Result<(), AppError> {
    let parent = old_path.parent().ok_or_else(|| AppError::io("no parent dir"))?;
    let old_name = file_name(old_path);
    let mut doc = read_doc(old_path)?;
    doc["name"] = json!(new_name);

    let slug = unique_slug(&slugify(new_name), &|s| {
        let fname = format!("{s}.json");
        !fname.eq_ignore_ascii_case(&old_name) && parent.join(&fname).exists()
    });
    let new_path = parent.join(format!("{slug}.json"));
    if new_path == old_path {
        write_json_atomic(old_path, &doc)?; // pure name-in-document change
    } else {
        let temp = parent.join(format!(".rk-tmp-{}.json", Uuid::new_v4()));
        fs::rename(old_path, &temp).map_err(|e| AppError::io(format!("rename failed: {e}")))?;
        write_json_atomic(&new_path, &doc)?;
        let _ = fs::remove_file(&temp);
    }
    Ok(())
}

fn read_doc(path: &Path) -> Result<Value, AppError> {
    let bytes = std::fs::read(path).map_err(|e| AppError::io(format!("could not read: {e}")))?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::io(format!("invalid JSON: {e}")))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn find_node(nodes: &[WorkspaceNode], id: &str) -> Option<WorkspaceNode> {
    for node in nodes {
        if node.id == id {
            return Some(node.clone());
        }
        if let Some(children) = &node.children {
            if let Some(found) = find_node(children, id) {
                return Some(found);
            }
        }
    }
    None
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

    #[test]
    fn create_collection_persists() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let node = ws.create_collection("My Col").unwrap();
        assert_eq!(node.name, "My Col");

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        assert_eq!(ws2.tree().len(), 1);
        assert_eq!(ws2.tree()[0].name, "My Col");
    }

    #[test]
    fn create_request_write_and_read_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let req = ws.create_request(&col.id, "Req").unwrap();

        let mut doc = ws.read_request(&req.id).unwrap();
        doc["method"] = json!("POST");
        ws.write_request(&req.id, doc).unwrap();

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        assert_eq!(ws2.read_request(&req.id).unwrap()["method"], "POST");
    }

    #[test]
    fn write_request_rejects_id_change() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let req = ws.create_request(&col.id, "Req").unwrap();

        let mut doc = ws.read_request(&req.id).unwrap();
        doc["id"] = json!("different");
        let err = ws.write_request(&req.id, doc).unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
    }

    #[test]
    fn rename_request_preserves_id() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let req = ws.create_request(&col.id, "Req").unwrap();

        let renamed = ws.rename_node(&req.id, "Renamed").unwrap();
        assert_eq!(renamed.id, req.id); // same identity
        assert_eq!(renamed.name, "Renamed");

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        assert_eq!(ws2.read_request(&req.id).unwrap()["name"], "Renamed");
    }

    #[test]
    fn rename_collection_moves_dir_and_keeps_id() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let req = ws.create_request(&col.id, "Req").unwrap();

        ws.rename_node(&col.id, "Renamed Col").unwrap();

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        let top = &ws2.tree()[0];
        assert_eq!(top.id, col.id);
        assert_eq!(top.name, "Renamed Col");
        // Child request survived the directory move with its id.
        assert!(ws2.path_for(&req.id).is_some());
    }

    #[test]
    fn delete_folder_removes_subtree() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let folder = ws.create_folder(&col.id, "Sub").unwrap();
        let req = ws.create_request(&folder.id, "Req").unwrap();

        ws.delete_node(&folder.id).unwrap();

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        assert!(ws2.path_for(&folder.id).is_none());
        assert!(ws2.path_for(&req.id).is_none());
        assert!(ws2.path_for(&col.id).is_some()); // collection intact
    }

    #[test]
    fn duplicate_request_makes_a_copy() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let req = ws.create_request(&col.id, "Req").unwrap();

        let dup = ws.duplicate_request(&req.id).unwrap();
        assert_ne!(dup.id, req.id);
        assert_eq!(dup.name, "Req copy");

        let (ws2, _) = Workspace::load(root.path().to_path_buf());
        assert!(ws2.path_for(&req.id).is_some()); // original intact
        assert!(ws2.path_for(&dup.id).is_some());
    }

    #[test]
    fn duplicate_rejects_containers() {
        let root = tempfile::tempdir().unwrap();
        let (mut ws, _) = Workspace::load(root.path().to_path_buf());
        let col = ws.create_collection("Col").unwrap();
        let err = ws.duplicate_request(&col.id).unwrap_err();
        assert_eq!(err.kind, ErrorKind::Validation);
    }
}
