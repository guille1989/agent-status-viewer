import type { PortInfo } from "../types";
import { PlugIcon } from "./Icons";

export function PortList({ ports }: { ports: PortInfo[] }) {
  return (
    <section className="section ports-section">
      <h2 className="section-title">Dispositivos detectados</h2>
      {ports.length === 0 ? (
        <div className="empty-state">
          <PlugIcon />
          <p>Comprueba que las impresoras estén encendidas y conectadas al equipo.</p>
        </div>
      ) : (
        <ul className="port-list">
          {ports.map((port) => (
            <li key={port.id} className="port-row">
              <span className="row-icon" aria-hidden="true">
                <PlugIcon />
              </span>
              <span className="port-copy">
                <span className="port-name">{port.name}</span>
                <span className="port-description">{port.description}</span>
              </span>
              <span className={`port-status status-${port.status}`}>
                <span className="status-dot" />
                {port.status === "active" ? "Conectado" : "En espera"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
