//! The workspace node tree (what the frontend renders) and the minimal typed
//! envelope Rust understands for each stored file. The full `RequestFile`
//! schema lives in the frontend TS type; Rust only parses `FileHeader` for the
//! index and round-trips the rest as `serde_json::Value` (design spec).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Collection,
    Folder,
    Request,
}

/// One node in the workspace tree, sent to the frontend (serde camelCase).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceNode {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    /// Collections/folders carry children; requests omit the field entirely.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<WorkspaceNode>>,
}

/// The typed header every stored document must have; parsed during a scan.
#[derive(Debug, Clone, Deserialize)]
pub struct FileHeader {
    pub version: u32,
    pub id: String,
    pub name: String,
}

/// The default document written for a freshly created request. Shape mirrors the
/// frontend `RequestFile` type so a round-trip needs no migration in M3a.
pub fn default_request_file(id: &str, name: &str) -> Value {
    json!({
        "version": 1,
        "id": id,
        "name": name,
        "method": "GET",
        "url": { "base": "", "query": [], "fragment": "" },
        "headers": [],
        "body": { "mode": "none" },
        "auth": { "type": "inherit" },
        "variables": [],
        "settings": { "timeoutMs": null, "followRedirects": null }
    })
}

/// `collection.json` contents.
pub fn collection_meta(id: &str, name: &str) -> Value {
    json!({
        "version": 1,
        "id": id,
        "name": name,
        "auth": { "type": "inherit" },
        "variables": []
    })
}

/// `folder.json` contents.
pub fn folder_meta(id: &str, name: &str) -> Value {
    json!({ "version": 1, "id": id, "name": name })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_request_file_round_trips_through_header() {
        let doc = default_request_file("id-1", "My request");
        let header: FileHeader = serde_json::from_value(doc.clone()).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.id, "id-1");
        assert_eq!(header.name, "My request");
        assert_eq!(doc["method"], "GET");
        assert_eq!(doc["body"]["mode"], "none");
        assert_eq!(doc["auth"]["type"], "inherit");
    }

    #[test]
    fn request_node_omits_children_key() {
        let node = WorkspaceNode {
            id: "r".into(),
            kind: NodeKind::Request,
            name: "r".into(),
            children: None,
        };
        let v = serde_json::to_value(&node).unwrap();
        assert!(v.get("children").is_none());
        assert_eq!(v["kind"], "request");
    }

    #[test]
    fn node_kind_serializes_camel() {
        assert_eq!(
            serde_json::to_value(NodeKind::Collection).unwrap(),
            "collection"
        );
        assert_eq!(serde_json::to_value(NodeKind::Folder).unwrap(), "folder");
        assert_eq!(serde_json::to_value(NodeKind::Request).unwrap(), "request");
    }
}
