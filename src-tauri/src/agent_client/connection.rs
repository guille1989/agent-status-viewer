use std::{fs, path::PathBuf, time::Duration};

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use super::protocol::StatusFile;
use crate::state::{ConnectionState, SharedState, MAX_RECENT_EVENTS};
use crate::tray;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Si el snapshot tiene más de esto, se considera al agente colgado/caído.
/// El pipe-server lo reescribe en cada escaneo de puertos (~10 s).
const STALE_AFTER_SECS: i64 = 40;

fn status_file_path() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\InnoApp Agent\status.json")
}

/// Poolea el archivo de estado que escribe el agente (que corre como
/// servicio LocalSystem). Reemplaza a la conexión por named pipe: un
/// proceso de usuario no puede abrir el pipe de un proceso SYSTEM.
pub fn spawn_connection_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            poll_once(&app);
            sleep(POLL_INTERVAL).await;
        }
    });
}

fn poll_once(app: &AppHandle) {
    let snapshot = read_status();

    let shared = app.state::<SharedState>();
    let mut guard = shared.0.lock().unwrap();

    match snapshot {
        Some(status) if !is_stale(&status.written_at) => {
            guard.connection = ConnectionState::Connected;
            guard.agent_name = status.agent.name;
            guard.agent_status = status.agent.status;
            guard.agent_status_message = status.status_message;
            guard.ports = status.ports;
            guard.recent_events = status.recent_events;
            guard.recent_events.truncate(MAX_RECENT_EVENTS);
        }
        _ => guard.mark_disconnected(),
    }

    let _ = app.emit("agent-state", &*guard);
    tray::update_tray(app, &guard);
}

fn read_status() -> Option<StatusFile> {
    let raw = fs::read_to_string(status_file_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// El agente escribe `writtenAt` en ISO 8601 (UTC, con `Z`). Sin traer una
/// dependencia de fechas, se compara la parte hasta los segundos con la
/// hora actual en UTC formateada igual.
fn is_stale(written_at: &str) -> bool {
    let elapsed = seconds_since_iso_utc(written_at);
    match elapsed {
        Some(secs) => secs > STALE_AFTER_SECS || secs < -STALE_AFTER_SECS,
        None => true,
    }
}

/// Segundos transcurridos desde `written_at` (ISO 8601 UTC) hasta ahora.
/// `None` si no se pudo parsear.
fn seconds_since_iso_utc(written_at: &str) -> Option<i64> {
    let ts = parse_iso_utc_to_epoch(written_at)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some(now - ts)
}

/// Parsea `YYYY-MM-DDTHH:MM:SS(.mmm)Z` a segundos epoch. Algoritmo de
/// días-desde-la-era civil (Howard Hinnant) para no depender de `chrono`.
fn parse_iso_utc_to_epoch(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let year = num(0, 4)?;
    let month = num(5, 7)?;
    let day = num(8, 10)?;
    let hour = num(11, 13)?;
    let minute = num(14, 16)?;
    let second = num(17, 19)?;

    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    Some(days * 86400 + hour * 3600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_of_unix_zero() {
        assert_eq!(parse_iso_utc_to_epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso_utc_to_epoch("1970-01-01T00:00:01.500Z"), Some(1));
        assert_eq!(parse_iso_utc_to_epoch("2000-01-01T00:00:00Z"), Some(946_684_800));
        // 2026-09-10T20:43:18Z  (verificado contra `date -u -d ... +%s`)
        assert_eq!(parse_iso_utc_to_epoch("2026-09-10T20:43:18.244Z"), Some(1_789_072_998));
    }

    #[test]
    fn parses_the_status_file_the_pipe_server_writes() {
        let json = r#"{
            "writtenAt": "2026-09-10T20:43:18.244Z",
            "agent": { "name": "print-capture-agent", "version": "0.3.0", "status": "error" },
            "statusMessage": "No se pudo conectar al backend",
            "ports": [
                { "id": "EPSON TM-T20II Receipt", "name": "EPSON TM-T20II Receipt",
                  "description": "Impresora (captura de spool)", "status": "active",
                  "lastActivityAt": "2026-09-10T20:43:18.243Z" }
            ],
            "recentEvents": [
                { "id": "abc-123", "timestamp": "2026-09-10T20:43:18.243Z", "port": "EPSON TM-T20II Receipt" }
            ]
        }"#;
        let parsed: StatusFile = serde_json::from_str(json).expect("debe parsear");
        assert_eq!(parsed.agent.name, "print-capture-agent");
        assert_eq!(parsed.agent.status, "error");
        assert_eq!(parsed.status_message.as_deref(), Some("No se pudo conectar al backend"));
        assert_eq!(parsed.ports.len(), 1);
        assert_eq!(parsed.ports[0].id, "EPSON TM-T20II Receipt");
        assert_eq!(parsed.recent_events.len(), 1);
    }

    #[test]
    fn stale_detection() {
        // Una fecha del pasado lejano siempre está vencida; una basura, también.
        assert!(is_stale("2020-01-01T00:00:00Z"));
        assert!(is_stale("no es una fecha"));

        // "Ahora" (± unos segundos por el tiempo de test) no está vencido.
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let elapsed = seconds_since_iso_utc("2020-01-01T00:00:00Z").unwrap();
        assert!(elapsed > 0 && elapsed == now_secs - 1_577_836_800);
    }
}
