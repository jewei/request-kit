//! Workspace scan: walk `collections/`, build the node tree the frontend
//! renders and the `id → path` index the backend resolves commands through.
//! Corrupt/unsupported files and duplicate embedded UUIDs are quarantined
//! deterministically (design spec / PLAN.md storage design).
//!
//! Two passes keep the logic simple:
//!   1. gather every parseable `(id, path)`; quarantine unparseable/too-new files.
//!   2. resolve duplicate ids (lexicographically-smallest relative path wins;
//!      the rest are quarantined), then build the tree from what remains on disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::storage::nodes::{FileHeader, NodeKind, WorkspaceNode};
use crate::storage::quarantine::{quarantine, QuarantineReport};

pub const COLLECTION_META: &str = "collection.json";
pub const FOLDER_META: &str = "folder.json";

#[derive(Debug, Default)]
pub struct ScanResult {
    pub tree: Vec<WorkspaceNode>,
    /// id → path. For requests this is the `.json` file; for collections and
    /// folders it is the directory.
    pub index: HashMap<String, PathBuf>,
    pub quarantined: Vec<QuarantineReport>,
}

/// Scans `collections_dir`. A missing directory yields an empty result.
pub fn scan(collections_dir: &Path) -> ScanResult {
    let mut result = ScanResult::default();
    if !collections_dir.exists() {
        return result;
    }

    // Pass 1: gather candidates; unparseable/too-new files are quarantined now.
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    gather(collections_dir, &mut candidates, &mut result.quarantined);

    // Resolve duplicate ids: smallest normalized relative path wins.
    let mut by_id: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (id, path) in candidates {
        by_id.entry(id).or_default().push(path);
    }
    for paths in by_id.values_mut() {
        if paths.len() > 1 {
            paths.sort_by_key(|p| norm_rel(p, collections_dir));
            for loser in paths.iter().skip(1) {
                result
                    .quarantined
                    .push(quarantine(loser, "duplicate UUID"));
            }
        }
    }

    // Pass 2: quarantined files are gone from disk now, so a fresh walk sees
    // only winners. Build the tree + index.
    for entry in sorted_dirs(collections_dir) {
        if let Some(node) = build_container(&entry, true, &mut result.index, &mut result.quarantined)
        {
            result.tree.push(node);
        }
    }
    result.tree.sort_by_key(|n| n.name.to_lowercase());
    result
}

/// Recursively collect `(id, path)` for every parseable JSON file; quarantine
/// files that fail to parse or declare an unsupported version.
fn gather(dir: &Path, out: &mut Vec<(String, PathBuf)>, quarantined: &mut Vec<QuarantineReport>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            gather(&path, out, quarantined);
        } else if is_json(&path) {
            match parse_header(&path) {
                Ok(header) => out.push((header.id, path)),
                Err(reason) => quarantined.push(quarantine(&path, reason)),
            }
        }
    }
}

/// Build a collection (or folder) node from `dir`; `None` if its metadata file
/// is missing/unreadable (reported, never deleted).
fn build_container(
    dir: &Path,
    is_collection: bool,
    index: &mut HashMap<String, PathBuf>,
    quarantined: &mut Vec<QuarantineReport>,
) -> Option<WorkspaceNode> {
    if !dir.is_dir() {
        return None;
    }
    let meta_name = if is_collection { COLLECTION_META } else { FOLDER_META };
    let meta_path = dir.join(meta_name);
    let header = match parse_header(&meta_path) {
        Ok(h) => h,
        Err(_) => {
            quarantined.push(QuarantineReport {
                original: meta_path.display().to_string(),
                reason: format!("{meta_name} missing or unreadable"),
            });
            return None;
        }
    };

    let mut children: Vec<WorkspaceNode> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(child) = build_container(&path, false, index, quarantined) {
                    children.push(child);
                }
            } else if is_json(&path) && path.file_name().and_then(|n| n.to_str()) != Some(meta_name)
            {
                if let Ok(req) = parse_header(&path) {
                    index.insert(req.id.clone(), path.clone());
                    children.push(WorkspaceNode {
                        id: req.id,
                        kind: NodeKind::Request,
                        name: req.name,
                        children: None,
                    });
                }
            }
        }
    }
    children.sort_by_key(|n| n.name.to_lowercase());

    index.insert(header.id.clone(), dir.to_path_buf());
    Some(WorkspaceNode {
        id: header.id,
        kind: if is_collection { NodeKind::Collection } else { NodeKind::Folder },
        name: header.name,
        children: Some(children),
    })
}

fn parse_header(path: &Path) -> Result<FileHeader, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not read file: {e}"))?;
    let header: FileHeader =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;
    if header.version > 1 {
        return Err(format!("unsupported version {}", header.version));
    }
    Ok(header)
}

fn is_json(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("json")
}

/// Directory entries of `dir`, sorted for deterministic iteration.
fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// Normalized `/`-separated path relative to `base` — the deterministic key for
/// duplicate-UUID tie-breaking (same winner on macOS and Windows).
fn norm_rel(path: &Path, base: &Path) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::atomic::write_json_atomic;
    use crate::storage::nodes::{collection_meta, default_request_file, folder_meta};

    fn write(path: &Path, value: &serde_json::Value) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_json_atomic(path, value).unwrap();
    }

    #[test]
    fn missing_dir_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan(&dir.path().join("collections"));
        assert!(result.tree.is_empty());
        assert!(result.index.is_empty());
    }

    #[test]
    fn builds_tree_and_index_alphabetically() {
        let root = tempfile::tempdir().unwrap();
        let col = root.path().join("my-col");
        write(&col.join(COLLECTION_META), &collection_meta("c1", "My Col"));
        write(&col.join("zeta.json"), &default_request_file("r2", "Zeta"));
        write(&col.join("alpha.json"), &default_request_file("r1", "Alpha"));

        let result = scan(root.path());
        assert_eq!(result.tree.len(), 1);
        let col_node = &result.tree[0];
        assert_eq!(col_node.id, "c1");
        let children = col_node.children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "Alpha"); // alphabetical by name
        assert_eq!(children[1].name, "Zeta");
        assert_eq!(result.index.len(), 3);
        assert!(result.index.contains_key("c1"));
        assert!(result.index.contains_key("r1"));
    }

    #[test]
    fn nests_folders() {
        let root = tempfile::tempdir().unwrap();
        let col = root.path().join("col");
        write(&col.join(COLLECTION_META), &collection_meta("c1", "Col"));
        let folder = col.join("sub");
        write(&folder.join(FOLDER_META), &folder_meta("f1", "Sub"));
        write(&folder.join("req.json"), &default_request_file("r1", "Req"));

        let result = scan(root.path());
        let col_node = &result.tree[0];
        let folder_node = &col_node.children.as_ref().unwrap()[0];
        assert_eq!(folder_node.kind, NodeKind::Folder);
        assert_eq!(folder_node.children.as_ref().unwrap()[0].id, "r1");
    }

    #[test]
    fn quarantines_corrupt_file() {
        let root = tempfile::tempdir().unwrap();
        let col = root.path().join("col");
        write(&col.join(COLLECTION_META), &collection_meta("c1", "Col"));
        std::fs::write(col.join("bad.json"), b"{ not json").unwrap();

        let result = scan(root.path());
        assert_eq!(result.quarantined.len(), 1);
        assert!(result.quarantined[0].original.ends_with("bad.json"));
        // The good collection still loads.
        assert_eq!(result.tree.len(), 1);
    }

    #[test]
    fn duplicate_uuid_keeps_lexicographically_first() {
        let root = tempfile::tempdir().unwrap();
        // Same id "dup" in collection "a" and collection "b".
        let a = root.path().join("a");
        write(&a.join(COLLECTION_META), &collection_meta("ca", "A"));
        write(&a.join("req.json"), &default_request_file("dup", "In A"));
        let b = root.path().join("b");
        write(&b.join(COLLECTION_META), &collection_meta("cb", "B"));
        write(&b.join("req.json"), &default_request_file("dup", "In B"));

        let result = scan(root.path());
        // a/req.json sorts before b/req.json → the A copy wins.
        assert_eq!(result.index.get("dup").unwrap(), &a.join("req.json"));
        assert_eq!(result.quarantined.len(), 1);
        assert!(result.quarantined[0].original.contains("b"));
    }

    #[test]
    fn rejects_unsupported_version() {
        let root = tempfile::tempdir().unwrap();
        let col = root.path().join("col");
        write(&col.join(COLLECTION_META), &collection_meta("c1", "Col"));
        write(
            &col.join("future.json"),
            &serde_json::json!({ "version": 2, "id": "r9", "name": "Future" }),
        );

        let result = scan(root.path());
        assert!(!result.index.contains_key("r9"));
        assert_eq!(result.quarantined.len(), 1);
        assert!(result.quarantined[0].reason.contains("unsupported version"));
    }
}
