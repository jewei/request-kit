// Modules are public so integration tests (tests/) can drive the executor
// directly without going through Tauri commands.
pub mod commands;
pub mod error;
pub mod http;
pub mod state;
pub mod storage;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        // Single-instance MUST be the first registered plugin so it runs
        // before any other plugin can act on behalf of a second process.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // The window is revealed explicitly after the frontend mounts;
                // letting the plugin restore visibility would cause a flash at
                // the wrong position.
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app::show_main_window,
            commands::app::storage_root,
            commands::http::send_request,
            commands::http::cancel_request,
            commands::http::release_response,
            commands::http::choose_and_save_response,
        ])
        .run(tauri::generate_context!())
        .expect("error while running request-kit");
}
