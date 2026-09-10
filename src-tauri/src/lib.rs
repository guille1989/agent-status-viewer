mod agent_client;
mod agent_runtime;
mod commands;
mod state;
mod tray;

use tauri::Manager;

use state::SharedState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // El agente lo corre el servicio Windows `innoapp-agent-service` (ver el
    // crate `agent-service`). Esta app es solo el visor de estado: se conecta
    // al named pipe del agente y, al activar, escribe `credentials.json` en
    // `C:\ProgramData\InnoApp Agent\` para que el servicio lo levante.
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(SharedState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            agent_runtime::get_activation_status,
            agent_runtime::activate_agent,
            agent_runtime::restart_agent,
        ])
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
        .expect("error while building tauri application");
}
