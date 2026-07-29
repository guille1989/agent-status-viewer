use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::time::sleep;

use super::protocol::{IncomingMessage, PIPE_NAME};
use crate::state::{ConnectionState, SharedState, MAX_RECENT_EVENTS};
use crate::tray;

const RETRY_DELAY: Duration = Duration::from_secs(3);
const PIPE_BUSY_RETRY_DELAY: Duration = Duration::from_millis(200);
const ERROR_PIPE_BUSY: i32 = 231;

/// Mantiene una conexión persistente al named pipe del agente, reconectando
/// automáticamente si el agente todavía no arrancó o si se cae la conexión.
pub fn spawn_connection_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = run_once(&app).await;
            broadcast_disconnected(&app);
            sleep(RETRY_DELAY).await;
        }
    });
}

async fn run_once(app: &AppHandle) -> std::io::Result<()> {
    let client = connect_with_retry().await?;

    {
        let shared = app.state::<SharedState>();
        let mut guard = shared.0.lock().unwrap();
        guard.connection = ConnectionState::Connected;
        broadcast(app, &guard);
    }

    let reader = BufReader::new(client);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(message) = serde_json::from_str::<IncomingMessage>(line) {
            apply_message(app, message);
        }
    }

    Ok(())
}

async fn connect_with_retry() -> std::io::Result<NamedPipeClient> {
    loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(client) => return Ok(client),
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                sleep(PIPE_BUSY_RETRY_DELAY).await;
            }
            Err(e) => return Err(e),
        }
    }
}

fn apply_message(app: &AppHandle, message: IncomingMessage) {
    let shared = app.state::<SharedState>();
    let mut guard = shared.0.lock().unwrap();

    match message {
        IncomingMessage::Snapshot { agent, ports, recent_events } => {
            guard.agent_name = agent.name;
            guard.agent_status = agent.status;
            guard.ports = ports;
            guard.recent_events = recent_events;
            guard.recent_events.truncate(MAX_RECENT_EVENTS);
        }
        IncomingMessage::PortUpdate { port } => guard.upsert_port(port),
        IncomingMessage::TicketEvent { event } => guard.push_event(event),
        IncomingMessage::AgentStatus { status, message } => {
            guard.agent_status = status;
            guard.agent_status_message = message;
        }
    }

    broadcast(app, &guard);
}

fn broadcast_disconnected(app: &AppHandle) {
    let shared = app.state::<SharedState>();
    let mut guard = shared.0.lock().unwrap();
    guard.mark_disconnected();
    broadcast(app, &guard);
}

fn broadcast(app: &AppHandle, state: &crate::state::AgentViewState) {
    let _ = app.emit("agent-state", state);
    tray::update_tray(app, state);
}
