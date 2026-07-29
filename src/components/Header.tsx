import type { AgentStatus, ConnectionState } from "../types";

interface HeaderProps {
  agentName: string;
  connection: ConnectionState;
  agentStatus: AgentStatus;
}

function statusLabel(connection: ConnectionState, agentStatus: AgentStatus): string {
  if (connection === "disconnected") return "Agente no detectado";
  if (agentStatus === "error") return "Error en el agente";
  if (agentStatus === "ok") return "Activo";
  return "Verificando...";
}

function statusDotClass(connection: ConnectionState, agentStatus: AgentStatus): string {
  if (connection === "disconnected") return "dot-gray";
  if (agentStatus === "error") return "dot-red";
  if (agentStatus === "ok") return "dot-green";
  return "dot-gray";
}

export function Header({ agentName, connection, agentStatus }: HeaderProps) {
  return (
    <header className="header">
      <span className="agent-name">{agentName}</span>
      <span className="status">
        <span className={`dot ${statusDotClass(connection, agentStatus)}`} />
        {statusLabel(connection, agentStatus)}
      </span>
    </header>
  );
}
