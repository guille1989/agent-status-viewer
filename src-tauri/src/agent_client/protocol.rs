use serde::Deserialize;

/// Nombre del named pipe que expone el print-capture-agent.
pub const PIPE_NAME: &str = r"\\.\pipe\print-capture-agent";

#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub version: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub last_activity_at: String,
}

/// El parseo del ticket (descripción, monto) vive del lado del servidor en
/// la nube, no en la PC del negocio — acá solo sabemos que se capturó algo.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketEvent {
    pub id: String,
    pub timestamp: String,
    pub port: String,
}

/// Un mensaje NDJSON emitido por el agente, una línea = un mensaje.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    Snapshot {
        agent: AgentInfo,
        ports: Vec<PortInfo>,
        #[serde(rename = "recentEvents")]
        recent_events: Vec<TicketEvent>,
    },
    PortUpdate {
        port: PortInfo,
    },
    TicketEvent {
        event: TicketEvent,
    },
    AgentStatus {
        status: String,
        message: Option<String>,
    },
}
