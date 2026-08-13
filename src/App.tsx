import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Header } from "./components/Header";
import { PortList } from "./components/PortList";
import { ActivityFeed } from "./components/ActivityFeed";
import { useAgentState } from "./hooks/useAgentState";
import "./styles.css";

interface ActivationStatus {
  activated: boolean;
  agentName?: string | null;
}

export default function App() {
  const state = useAgentState();
  const [activation, setActivation] = useState<ActivationStatus | null>(null);
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    invoke<ActivationStatus>("get_activation_status")
      .then(setActivation)
      .catch((err) => setError(String(err)));
  }, []);

  async function activate(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const next = await invoke<ActivationStatus>("activate_agent", {
        code,
        name: name.trim() || "Agente piloto",
      });
      setActivation(next);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  if (!activation) {
    return (
      <div className="activation-shell">
        <div className="ambient-glow" />
        <img src="/innoapp-mark.png" alt="InnoApp" />
        <span className="activation-spinner" />
        {error && <p className="activation-error">{error}</p>}
      </div>
    );
  }

  if (!activation.activated) {
    return (
      <main className="activation-screen">
        <div className="activation-glow" />
        <div className="activation-brand">
          <img src="/innoapp-mark.png" alt="" />
          <strong>Inno<span>App</span></strong>
        </div>
        <div className="activation-copy">
          <span>Programa piloto</span>
          <h1>Activa este agente</h1>
          <p>Copia el código que aparece en la sección Agentes de InnoApp Web.</p>
        </div>
        <form onSubmit={activate}>
          <label>
            Código de activación
            <input value={code} onChange={(e) => setCode(e.target.value.toUpperCase())} required minLength={10} placeholder="XXXXX-XXXXX" autoFocus />
          </label>
          <label>
            Nombre del equipo
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Ej. Caja principal" />
          </label>
          {error && <div className="activation-error" role="alert">{error}</div>}
          <button type="submit" disabled={submitting}>{submitting ? "Activando…" : "Activar agente"}</button>
        </form>
        <small>La captura permanece desactivada durante la fase inicial del piloto.</small>
      </main>
    );
  }

  return (
    <div className="app">
      <div className="ambient-glow" />
      <Header
        agentName={activation.agentName || state.agentName}
        connection={state.connection}
        agentStatus={state.agentStatus}
        agentStatusMessage={state.agentStatusMessage}
        detectedPorts={state.ports.length}
      />
      {state.connection === "disconnected" ? (
        <div className="disconnected-banner">
          <p className="disconnected-title">El agente aún no responde</p>
          <p className="disconnected-hint">Puedes volver a intentar la conexión.</p>
          <button className="restart-button" onClick={() => void invoke("restart_agent")}>Reintentar</button>
        </div>
      ) : (
        <>
          <PortList ports={state.ports} />
          <ActivityFeed events={state.recentEvents} />
        </>
      )}
    </div>
  );
}
