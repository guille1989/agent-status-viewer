import type { AgentStatus, ConnectionState } from "../types";
import { AlertIcon, CheckIcon, LoaderIcon } from "./Icons";

interface HeaderProps {
  agentName: string;
  connection: ConnectionState;
  agentStatus: AgentStatus;
  agentStatusMessage: string | null;
  detectedPorts: number;
}

type VisualTone = "success" | "warning" | "danger" | "neutral";

interface StatusContent {
  badge: string;
  title: string;
  description: string;
  tone: VisualTone;
}

function getStatusContent(
  connection: ConnectionState,
  agentStatus: AgentStatus,
  agentStatusMessage: string | null,
  detectedPorts: number,
): StatusContent {
  if (connection === "disconnected") {
    return {
      badge: "Desconectado",
      title: "Esperando al agente",
      description: "La primera conexión puede tardar unos segundos.",
      tone: "neutral",
    };
  }

  if (agentStatus === "error") {
    return {
      badge: "Con error",
      title: "El agente necesita atención",
      description: agentStatusMessage || "Se detectó un problema durante la captura de tickets.",
      tone: "danger",
    };
  }

  if (agentStatus === "unknown") {
    return {
      badge: "Verificando",
      title: "Comprobando el estado",
      description: "Estamos validando la conexión y los puertos disponibles.",
      tone: "neutral",
    };
  }

  if (detectedPorts === 0) {
    return {
      badge: "Activo",
      title: "No se detectan puertos",
      description: "El agente está activo, pero todavía no encuentra puertos conectados.",
      tone: "warning",
    };
  }

  return {
    badge: "Activo",
    title: "Todo funciona correctamente",
    description: "Los tickets se están capturando y enviando sin problemas.",
    tone: "success",
  };
}

export function Header({
  agentName,
  connection,
  agentStatus,
  agentStatusMessage,
  detectedPorts,
}: HeaderProps) {
  const content = getStatusContent(
    connection,
    agentStatus,
    agentStatusMessage,
    detectedPorts,
  );
  const SummaryIcon =
    content.tone === "success"
      ? CheckIcon
      : content.tone === "neutral"
        ? LoaderIcon
        : AlertIcon;

  return (
    <header className="agent-header">
      <div className="brand-row">
        <img src="/innoapp-mark.png" alt="" className="brand-mark" />
        <span>Estado del agente</span>
      </div>

      <div className="identity-row">
        <div className="agent-identity">
          <span className="eyebrow">Agente</span>
          <h1>{agentName}</h1>
        </div>
        <span className={`status-badge tone-${content.tone}`}>
          <span className="status-dot" />
          {content.badge}
        </span>
      </div>

      <div className={`summary-card tone-${content.tone}`}>
        <SummaryIcon className="summary-icon" />
        <div>
          <p className="summary-title">{content.title}</p>
          <p className="summary-description">{content.description}</p>
        </div>
      </div>
    </header>
  );
}
