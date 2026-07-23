//! Storage command surface (M2a). Each `#[tauri::command]` is a thin wrapper
//! over a plain `_impl(&AppState, …)` function so the logic is testable without
//! a Tauri runtime and without touching the real `~/.request-kit`.
//!
//! JS passes camelCase args (`parentId`); Tauri maps them to snake_case params.

use std::path::PathBuf;
use std::sync::PoisonError;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

use crate::error::{AppError, ErrorKind};
use crate::state::{AppMode, AppState};
use crate::storage::nodes::WorkspaceNode;
use crate::storage::paths::ensure_storage_root;
use crate::storage::quarantine::QuarantineReport;
use crate::storage::workspace::Workspace;

/// One bootstrap payload (design spec: M2a fills only `tree`/`quarantined`;
/// environments/globals/settings/uiState are M2b and return defaults).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBootstrap {
    pub tree: Vec<WorkspaceNode>,
    pub environments: Vec<Value>,
    pub globals: Vec<Value>,
    pub settings: Value,
    pub ui_state: Value,
    pub quarantined: Vec<QuarantineReport>,
}

fn default_settings() -> Value {
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

fn default_ui_state() -> Value {
    json!({
        "version": 1,
        "openTabs": [],
        "activeTabId": null,
        "activeEnvId": null,
        "sidebarWidth": 260,
        "sidebarVisible": true,
        "expandedFolderIds": []
    })
}

/// Storage mutations are refused while an import is being applied/recovered.
fn ensure_normal(state: &AppState) -> Result<(), AppError> {
    let mode = *state.mode.lock().unwrap_or_else(PoisonError::into_inner);
    if mode != AppMode::Normal {
        return Err(AppError::new(
            ErrorKind::MaintenanceInProgress,
            "storage is busy applying an import — try again in a moment",
        ));
    }
    Ok(())
}

/// Runs `f` against the loaded workspace, erroring if none is loaded yet.
fn with_workspace<T>(
    state: &AppState,
    f: impl FnOnce(&mut Workspace) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let mut guard = state
        .workspace
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let ws = guard
        .as_mut()
        .ok_or_else(|| AppError::new(ErrorKind::Validation, "workspace is not loaded"))?;
    f(ws)
}

// --- logic (testable) ---

pub fn load_workspace_impl(
    state: &AppState,
    root: PathBuf,
) -> Result<WorkspaceBootstrap, AppError> {
    let (ws, quarantined) = Workspace::load(root);
    let tree = ws.tree();
    *state
        .workspace
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(ws);
    Ok(WorkspaceBootstrap {
        tree,
        environments: vec![],
        globals: vec![],
        settings: default_settings(),
        ui_state: default_ui_state(),
        quarantined,
    })
}

pub fn create_collection_impl(state: &AppState, name: &str) -> Result<WorkspaceNode, AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.create_collection(name))
}

pub fn create_folder_impl(
    state: &AppState,
    parent_id: &str,
    name: &str,
) -> Result<WorkspaceNode, AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.create_folder(parent_id, name))
}

pub fn create_request_impl(
    state: &AppState,
    parent_id: &str,
    name: &str,
) -> Result<WorkspaceNode, AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.create_request(parent_id, name))
}

pub fn read_request_impl(state: &AppState, id: &str) -> Result<Value, AppError> {
    with_workspace(state, |ws| ws.read_request(id))
}

pub fn write_request_impl(state: &AppState, id: &str, document: Value) -> Result<(), AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.write_request(id, document))
}

pub fn rename_node_impl(
    state: &AppState,
    id: &str,
    new_name: &str,
) -> Result<WorkspaceNode, AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.rename_node(id, new_name))
}

pub fn delete_node_impl(state: &AppState, id: &str) -> Result<(), AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.delete_node(id))
}

pub fn duplicate_request_impl(state: &AppState, id: &str) -> Result<WorkspaceNode, AppError> {
    ensure_normal(state)?;
    with_workspace(state, |ws| ws.duplicate_request(id))
}

// --- Tauri command wrappers ---

#[tauri::command]
pub fn load_workspace(state: State<'_, AppState>) -> Result<WorkspaceBootstrap, AppError> {
    load_workspace_impl(state.inner(), ensure_storage_root()?)
}

#[tauri::command]
pub fn create_collection(
    name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceNode, AppError> {
    create_collection_impl(state.inner(), &name)
}

#[tauri::command]
pub fn create_folder(
    parent_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceNode, AppError> {
    create_folder_impl(state.inner(), &parent_id, &name)
}

#[tauri::command]
pub fn create_request(
    parent_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceNode, AppError> {
    create_request_impl(state.inner(), &parent_id, &name)
}

#[tauri::command]
pub fn read_request(id: String, state: State<'_, AppState>) -> Result<Value, AppError> {
    read_request_impl(state.inner(), &id)
}

#[tauri::command]
pub fn write_request(
    id: String,
    document: Value,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    write_request_impl(state.inner(), &id, document)
}

#[tauri::command]
pub fn rename_node(
    id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceNode, AppError> {
    rename_node_impl(state.inner(), &id, &new_name)
}

#[tauri::command]
pub fn delete_node(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    delete_node_impl(state.inner(), &id)
}

#[tauri::command]
pub fn duplicate_request(
    id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceNode, AppError> {
    duplicate_request_impl(state.inner(), &id)
}
