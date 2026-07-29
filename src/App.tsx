import { Header } from "./components/Header";
import { PortList } from "./components/PortList";
import { ActivityFeed } from "./components/ActivityFeed";
import { useAgentState } from "./hooks/useAgentState";
import "./styles.css";

export default function App() {
  const state = useAgentState();

  return (
    <div className="app">
      <Header
        agentName={state.agentName}
        connection={state.connection}
        agentStatus={state.agentStatus}
      />
      {state.connection === "disconnected" ? (
        <div className="disconnected-banner">
          <p className="disconnected-title">Agente no detectado</p>
          <p className="disconnected-hint">
            Verificá que el servicio print-capture-agent esté corriendo.
          </p>
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
