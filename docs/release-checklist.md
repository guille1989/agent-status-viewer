# Checklist de release — InnoApp Agent

Evita publicar setup / web con números desfasados (Fase 0 OTA).

## Tabla: qué versión se reporta dónde

| Superficie | Fuente de verdad | Quién la ve |
|------------|------------------|-------------|
| **Agent JS** (heartbeat `version`, pipe, `status.json`) | `print-capture-agent/package.json` → inyectada en `app.mjs` como `AGENT_VERSION` | Cloud (`Agents.version`), admin-console, dashboard robots |
| **Producto Windows** (nombre del setup NSIS) | `agent-status-viewer/package.json` = `src-tauri/tauri.conf.json` = `src-tauri/Cargo.toml` | Negocio al descargar el `.exe` |
| **Marketing web** | `VITE_AGENT_VERSION` en deploy de `innoapp-web-user-client` (default alineado al producto) | Onboarding / pestaña Instalar |
| **Servicio Rust** | `agent-service/Cargo.toml` | Ops / runbook; **no** se reporta al cloud hoy |
| **Pipe lib** | `print-capture-agent-pipe-server/package.json` | Solo versión de librería; va inlined en `app.mjs` |

Regla: en un release de **setup**, agent JS = producto Windows = `VITE_AGENT_VERSION`.
El servicio puede quedar atrás si el release no lo toca; si el release **exige**
servicio ≥ X, anotarlo abajo y bumpear `agent-service` en el mismo setup.

## Antes de `npm run build:pilot`

- [ ] Bumpear juntos: `print-capture-agent/package.json`, `agent-status-viewer/package.json`, `tauri.conf.json`, `src-tauri/Cargo.toml`.
- [ ] `prepare-pilot.ps1` falla si esas versiones no coinciden (no saltarse el script).
- [ ] ¿Cambia spawn/env/watchdog del servicio? → bumpear `agent-service/Cargo.toml` y anotar `minServiceVersion` en notas del release.
- [ ] ¿Cambia Node / `serialport` / ABI? → no es hotfix JS; hay que rebundlear runtime completo (no OTA `app.mjs` solo).
- [ ] ¿Cambia bandeja / activación / pipe client? → va en este setup; no se cubre con OTA JS.

## Artefacto OTA JS (cuando exista pipeline; Fase 0 = hosting listo)

Prefijo S3 (cloud): `agent-releases/<version>/`

| Objeto | Contenido |
|--------|-----------|
| `app.mjs` | Bundle esbuild del agent |
| `app.mjs.sha256` | Hex SHA-256 del bundle (una línea) |
| `manifest.json` | `{ version, sha256, url?, minServiceVersion?, releasedAt }` — **inmutable** por versión |

Integridad Fase 0→1: **HTTPS + SHA-256** publicado junto al artefacto (y/o en
el manifiesto). Firma asimétrica del manifiesto / Authenticode del setup =
después (Fase 3 / endurecimiento). Script: `ticket-parsing-cloud` →
`npm run publish-agent-release`.

## Después del build

- [ ] Setup: `src-tauri/target/release/bundle/nsis/InnoApp Agent_<versión>_x64-setup.exe`
- [ ] Publicar instalador donde apunte `VITE_AGENT_DOWNLOAD_URL`.
- [ ] Deploy web con `VITE_AGENT_VERSION=<misma versión>`.
- [ ] (Opcional OTA) `publish-agent-release` del `app.mjs` al bucket de releases.
- [ ] Verificar en admin: un agent de lab reporta la nueva `version` tras install/swap.

## Paths

Ver [`paths.md`](./paths.md).
