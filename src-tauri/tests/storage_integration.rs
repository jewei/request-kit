//! Command-level storage lifecycle, driven through the `_impl` functions with a
//! real `AppState` pointed at a tempdir (no Tauri runtime, no `~/.request-kit`).
//! The heavy per-operation logic is unit-tested in `storage::workspace`; this
//! proves the command layer composes and the maintenance guard bites.

use request_kit_lib::commands::storage::{
    create_collection_impl, create_request_impl, delete_node_impl, duplicate_request_impl,
    load_workspace_impl, read_request_impl, rename_node_impl, write_request_impl,
};
use request_kit_lib::error::ErrorKind;
use request_kit_lib::state::{AppMode, AppState};

#[test]
fn full_lifecycle_create_save_reload_rename_delete() {
    let root = tempfile::tempdir().unwrap();
    let state = AppState::default();

    // Load an empty workspace.
    let boot = load_workspace_impl(&state, root.path().to_path_buf()).unwrap();
    assert!(boot.tree.is_empty());
    assert!(boot.quarantined.is_empty());

    // Create a collection + request.
    let col = create_collection_impl(&state, "My API").unwrap();
    let req = create_request_impl(&state, &col.id, "Ping").unwrap();

    // Edit + save the request.
    let mut doc = read_request_impl(&state, &req.id).unwrap();
    doc["method"] = serde_json::json!("POST");
    write_request_impl(&state, &req.id, doc).unwrap();

    // Reload from disk into a fresh state — the edit survived.
    let state2 = AppState::default();
    load_workspace_impl(&state2, root.path().to_path_buf()).unwrap();
    assert_eq!(
        read_request_impl(&state2, &req.id).unwrap()["method"],
        "POST"
    );

    // Rename keeps identity.
    let renamed = rename_node_impl(&state2, &req.id, "Ping v2").unwrap();
    assert_eq!(renamed.id, req.id);
    assert_eq!(renamed.name, "Ping v2");

    // Duplicate then delete the original; duplicate remains.
    let dup = duplicate_request_impl(&state2, &req.id).unwrap();
    delete_node_impl(&state2, &req.id).unwrap();

    let state3 = AppState::default();
    let boot3 = load_workspace_impl(&state3, root.path().to_path_buf()).unwrap();
    assert_eq!(boot3.tree.len(), 1);
    assert!(read_request_impl(&state3, &req.id).is_err()); // original gone
    assert!(read_request_impl(&state3, &dup.id).is_ok()); // copy remains
}

#[test]
fn mutations_rejected_during_import() {
    let root = tempfile::tempdir().unwrap();
    let state = AppState::default();
    load_workspace_impl(&state, root.path().to_path_buf()).unwrap();

    *state.mode.lock().unwrap() = AppMode::ImportApplying;

    let err = create_collection_impl(&state, "Nope").unwrap_err();
    assert_eq!(err.kind, ErrorKind::MaintenanceInProgress);
}
