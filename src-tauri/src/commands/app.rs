use tauri::Manager;

use crate::error::AppError;
use crate::storage::paths;

/// Reveal the main window after the frontend has mounted. The window starts
/// hidden (`visible: false`) so window-state can restore geometry first.
#[tauri::command]
pub fn show_main_window(window: tauri::Window) -> Result<(), AppError> {
    if let Some(main) = window.app_handle().get_webview_window("main") {
        main.show().map_err(|e| AppError::io(e.to_string()))?;
        main.set_focus().map_err(|e| AppError::io(e.to_string()))?;
    }
    Ok(())
}

/// Resolved storage root as a display string (M0 smoke check; also ensures the
/// directory exists with restrictive permissions).
#[tauri::command]
pub fn storage_root() -> Result<String, AppError> {
    let root = paths::ensure_storage_root()?;
    Ok(root.display().to_string())
}
