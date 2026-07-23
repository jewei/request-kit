use std::sync::PoisonError;

use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::error::{AppError, ErrorKind};
use crate::http::executor;
use crate::http::types::{HttpResponseData, SendRequestPayload};
use crate::state::AppState;

#[tauri::command]
pub async fn send_request(
    payload: SendRequestPayload,
    state: State<'_, AppState>,
) -> Result<HttpResponseData, AppError> {
    executor::send(&state, payload).await
}

/// No-op if the execution already finished (its token is gone by then).
#[tauri::command]
pub fn cancel_request(execution_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    if let Some(token) = state
        .abort_map
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&execution_id)
    {
        token.cancel();
    }
    Ok(())
}

/// Idempotent — releasing an unknown or already-released id succeeds silently.
#[tauri::command]
pub fn release_response(execution_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    state.retention.release(&execution_id);
    Ok(())
}

/// Rust-owned save dialog (PLAN.md: arbitrary paths never cross IPC). Returns
/// `false` when the user cancels the dialog.
#[tauri::command]
pub async fn choose_and_save_response(
    execution_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let Some(bytes) = state.retention.get(&execution_id) else {
        return Err(AppError::new(
            ErrorKind::Validation,
            "this response is no longer retained — send the request again to save it",
        ));
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name("response.txt")
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx
        .await
        .map_err(|_| AppError::io("the save dialog closed unexpectedly"))?;
    let Some(file_path) = chosen else {
        return Ok(false);
    };
    let path = file_path
        .into_path()
        .map_err(|e| AppError::io(e.to_string()))?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| AppError::io(format!("could not write the file: {e}")))?;
    Ok(true)
}
