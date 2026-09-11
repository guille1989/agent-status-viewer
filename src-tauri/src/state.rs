use std::sync::Mutex;

use serde::Serialize;

use crate::agent_client::protocol::{PortInfo, TicketEvent};

/// Cantidad máxima de eventos que se conservan para el feed de actividad.
pub const MAX_RECENT_EVENTS: usize = 6;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentViewState {
    pub connection: ConnectionState,
    pub agent_name: String,
    pub agent_status: String, // "ok" | "error" | "unknown"
    pub agent_status_message: Option<String>,
    pub ports: Vec<PortInfo>,
    pub recent_events: Vec<TicketEvent>, // más reciente primero
}

impl Default for AgentViewState {
    fn default() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            agent_name: "print-capture-agent".to_string(),
            agent_status: "unknown".to_string(),
            agent_status_message: None,
            ports: Vec::new(),
            recent_events: Vec::new(),
        }
    }
}

impl AgentViewState {
    pub fn mark_disconnected(&mut self) {
        self.connection = ConnectionState::Disconnected;
        self.agent_status = "unknown".to_string();
        self.agent_status_message = None;
    }
}

pub struct SharedState(pub Mutex<AgentViewState>);

impl SharedState {
    pub fn new() -> Self {
        Self(Mutex::new(AgentViewState::default()))
    }
}
