use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    #[serde(default)]
    #[allow(dead_code)]
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

/// Snapshot del estado del agente, tal como el pipe-server lo escribe en
/// `C:\ProgramData\InnoApp Agent\status.json`. La app de bandeja lo poolea
/// en vez de abrir el named pipe (que un proceso de LocalSystem no comparte
/// con un proceso de usuario).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusFile {
    pub written_at: String,
    pub agent: AgentInfo,
    #[serde(default)]
    pub status_message: Option<String>,
    #[serde(default)]
    pub ports: Vec<PortInfo>,
    #[serde(default)]
    pub recent_events: Vec<TicketEvent>,
}
