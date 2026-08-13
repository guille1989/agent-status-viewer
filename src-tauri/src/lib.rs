mod agent_client;
mod agent_runtime;
mod commands;
mod state;
mod tray;

use tauri::Manager;

use state::SharedState;
use agent_runtime::AgentProcess;

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
        .manage(AgentProcess::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            agent_runtime::get_activation_status,
            agent_runtime::activate_agent,
            agent_runtime::restart_agent,
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            if let Some(process) = app.try_state::<AgentProcess>() {
                if let Err(err) = agent_runtime::start_agent(app.handle(), &process) {
                    eprintln!("[runtime] {err}");
                }
            }
            agent_client::connection::spawn_connection_loop(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(process) = app.try_state::<AgentProcess>() {
                    agent_runtime::stop_agent(&process);
                }
            }
        });
}
