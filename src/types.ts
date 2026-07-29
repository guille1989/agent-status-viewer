export type ConnectionState = "connected" | "disconnected";
export type AgentStatus = "ok" | "error" | "unknown";
export type PortStatus = "active" | "idle";

export interface PortInfo {
  id: string;
  name: string;
  description: string;
  status: PortStatus;
  lastActivityAt: string;
}

// El parseo (descripción, monto) vive en el servidor de la nube, no acá —
// el viewer solo sabe que se capturó un ticket, no qué contiene.
export interface TicketEvent {
  id: string;
  timestamp: string;
  port: string;
}

export interface AgentViewState {
  connection: ConnectionState;
  agentName: string;
  agentStatus: AgentStatus;
  agentStatusMessage: string | null;
  ports: PortInfo[];
  recentEvents: TicketEvent[];
}
