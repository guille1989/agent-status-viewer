# InnoApp Agent

Dos piezas:

- **`agent-service/`** — servicio Windows (`innoapp-agent-service`, corre como
  `LocalSystem`) que ejecuta el `print-capture-agent`. Arranca solo con
  Windows, sobrevive reboot / cierre de sesión, y tiene permiso para leer
  `spool\PRINTERS` (la captura de spool lo necesita).
- **`src-tauri/`** — app de bandeja Tauri. **Solo visor de estado**: se
  conecta al named pipe `\\.\pipe\print-capture-agent` y muestra el estado.
  Al activar, escribe `credentials.json` en `C:\ProgramData\InnoApp Agent\`;
  el servicio detecta el cambio y (re)arranca el agente.

## Desarrollo

```powershell
npm install
npm run tauri dev            # la app de bandeja
cargo run --manifest-path agent-service/Cargo.toml install    # instala+arranca el servicio (admin)
cargo run --manifest-path agent-service/Cargo.toml uninstall  # lo saca
```

## Instalador piloto

Empaqueta el servicio, el `print-capture-agent`, un runtime Node y las
dependencias nativas de `serialport`. El usuario final no instala nada más.

```powershell
npm run build:pilot
```

Sale en:

```text
src-tauri/target/release/bundle/nsis/InnoApp Agent_0.3.0_x64-setup.exe
```

El instalador (elevado, `installMode: perMachine`):
1. copia todo a `C:\Program Files\InnoApp Agent\`
2. crea `C:\ProgramData\InnoApp Agent\` con permiso de modificación para
   `BUILTIN\Users` (para que la activación escriba `credentials.json` sin
   elevar)
3. registra y arranca el servicio `InnoAppAgent` (`sc create ... start= auto`,
   con acciones de recuperación)
4. deja un acceso directo a la app de bandeja en el arranque de todos los
   usuarios

Ver [`src-tauri/installer-hooks.nsh`](src-tauri/installer-hooks.nsh).

### Captura

Viene **prendida** (`ENABLE_CAPTURE=true`, lo setea el servicio). Para dejar
una instalación en modo "solo diagnóstico": definir `ENABLE_CAPTURE=false`
como variable de entorno **de la máquina** y reiniciar el servicio
(`sc stop InnoAppAgent && sc start InnoAppAgent`).

### Datos y logs

Todo en `C:\ProgramData\InnoApp Agent\`:
`credentials.json`, `queue.json`, `logs\agent.log`, `logs\agent-error.log`,
`logs\service.log`.

El script reproducible que prepara el runtime + compila el servicio es
[`scripts/prepare-pilot.ps1`](scripts/prepare-pilot.ps1).
