import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentViewState } from "../types";

const INITIAL_STATE: AgentViewState = {
  connection: "disconnected",
  agentName: "print-capture-agent",
  agentStatus: "unknown",
  agentStatusMessage: null,
  ports: [],
  recentEvents: [],
};

/**
 * El backend Rust mantiene la conexión al agente y empuja el estado completo
 * cada vez que cambia. Acá solo pedimos el snapshot actual al montar y nos
 * suscribimos a las actualizaciones subsiguientes.
 */
export function useAgentState(): AgentViewState {
  const [state, setState] = useState<AgentViewState>(INITIAL_STATE);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    invoke<AgentViewState>("get_status").then((initial) => {
      if (!cancelled) setState(initial);
    });

    listen<AgentViewState>("agent-state", (event) => {
      setState(event.payload);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return state;
}
