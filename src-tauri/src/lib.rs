mod agent_client;
mod commands;
mod state;
mod tray;

use tauri::Manager;

use state::SharedState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(SharedState::new())
        .invoke_handler(tauri::generate_handler![commands::get_status])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            agent_client::connection::spawn_connection_loop(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
