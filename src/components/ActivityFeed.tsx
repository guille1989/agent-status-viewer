import { useLayoutEffect, useRef } from "react";
import type { TicketEvent } from "../types";
import { TicketIcon } from "./Icons";

function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleTimeString("es-AR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/**
 * Anima el desplazamiento de las filas existentes cuando entra un ticket
 * nuevo. Las filas nuevas usan el keyframe `slide-in` de styles.css.
 */
function useRowShiftAnimation(rowIds: string[]) {
  const rowRefs = useRef(new Map<string, HTMLLIElement>());
  const prevTops = useRef(new Map<string, number>());

  useLayoutEffect(() => {
    const nextTops = new Map<string, number>();
    rowRefs.current.forEach((el, id) => {
      nextTops.set(id, el.getBoundingClientRect().top);
    });

    rowRefs.current.forEach((el, id) => {
      const prevTop = prevTops.current.get(id);
      const nextTop = nextTops.get(id);
      if (prevTop === undefined || nextTop === undefined) return;
      const delta = prevTop - nextTop;
      if (delta === 0) return;

      el.style.transition = "none";
      el.style.transform = `translateY(${delta}px)`;
      requestAnimationFrame(() => {
        el.style.transition = "transform 0.28s cubic-bezier(0.16, 1, 0.3, 1)";
        el.style.transform = "";
      });
    });

    prevTops.current = nextTops;
  }, [rowIds.join(",")]);

  return rowRefs.current;
}

export function ActivityFeed({ events }: { events: TicketEvent[] }) {
  const rowRefs = useRowShiftAnimation(events.map((event) => event.id));

  return (
    <section className="section section-grow">
      <h2 className="section-title">Actividad reciente</h2>
      {events.length === 0 ? (
        <div className="empty-state compact">
          <TicketIcon />
          <p>Todavía no se capturaron tickets.</p>
        </div>
      ) : (
        <ul className="activity-list">
          {events.map((event) => (
            <li
              key={event.id}
              className="activity-row"
              ref={(el) => {
                if (el) rowRefs.set(event.id, el);
                else rowRefs.delete(event.id);
              }}
            >
              <span className="activity-icon" aria-hidden="true"><TicketIcon /></span>
              <div className="activity-main">
                <span className="activity-description">Ticket capturado</span>
                <span className="activity-meta">{event.port}</span>
              </div>
              <time className="activity-time" dateTime={event.timestamp}>
                {formatTime(event.timestamp)}
              </time>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
