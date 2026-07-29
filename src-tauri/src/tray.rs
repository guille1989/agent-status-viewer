use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::state::{AgentViewState, ConnectionState};

const TRAY_ID: &str = "main-tray";

const ICON_OK: &[u8] = include_bytes!("../icons/tray-ok.png");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray-error.png");
const ICON_DISCONNECTED: &[u8] = include_bytes!("../icons/tray-disconnected.png");

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit_item = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON_DISCONNECTED)?)
        .tooltip("Agente no detectado")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Refleja el estado actual del agente en el ícono/tooltip de la bandeja,
/// para que se pueda ver de un vistazo sin abrir la ventana.
pub fn update_tray(app: &AppHandle, state: &AgentViewState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let (icon_bytes, tooltip) = match state.connection {
        ConnectionState::Disconnected => {
            (ICON_DISCONNECTED, "Agente no detectado".to_string())
        }
        ConnectionState::Connected => match state.agent_status.as_str() {
            "error" => (
                ICON_ERROR,
                state
                    .agent_status_message
                    .clone()
                    .unwrap_or_else(|| "El agente reportó un error".to_string()),
            ),
            "ok" => (ICON_OK, "Agente funcionando correctamente".to_string()),
            _ => (
                ICON_DISCONNECTED,
                "Verificando estado del agente...".to_string(),
            ),
        },
    };

    if let Ok(icon) = Image::from_bytes(icon_bytes) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(tooltip.as_str()));
}
