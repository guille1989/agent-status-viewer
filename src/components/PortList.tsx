import type { PortInfo } from "../types";

export function PortList({ ports }: { ports: PortInfo[] }) {
  return (
    <section className="section">
      <h2 className="section-title">Puertos detectados</h2>
      {ports.length === 0 ? (
        <p className="empty">Todavía no se detectaron puertos.</p>
      ) : (
        <ul className="port-list">
          {ports.map((port) => (
            <li key={port.id} className="port-row">
              <span className={`dot ${port.status === "active" ? "dot-green" : "dot-gray"}`} />
              <span className="port-name">{port.name}</span>
              <span className="port-description">{port.description}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
