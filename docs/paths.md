# Paths canónicos — InnoApp Agent (Windows)

Único layout de producto (piloto NSIS `perMachine`). No usar AppData del
usuario para runtime ni para credenciales nuevas.

## Program Files (runtime)

```text
C:\Program Files\InnoApp Agent\
  InnoApp Agent.exe
  resources\
    innoapp-agent-service.exe
    agent\
      node.exe
      app.mjs
      package.json
      node_modules\
```

Escritura requiere elevación / LocalSystem. El swap OTA de `app.mjs` (Fase 1)
debe orquestarse desde el servicio o el hijo Node, **nunca** desde la bandeja.

## ProgramData (estado / staging)

```text
C:\ProgramData\InnoApp Agent\
  credentials.json
  queue.json
  status.json
  logs\
  updates\
    staging\app.mjs
    pending-apply.json      # agent → servicio: swap + reinicio
    boot-check.json         # post-swap; agent OK lo borra
    rolled-back.json        # servicio restauró .prev
    app.mjs.prev
```

Se conserva en uninstall/reinstall a propósito (activación y cola).

## Servicio + OTA (Fase 1)

- Nombre SCM: `InnoAppAgent`
- Display: `InnoApp Agent`
- Cuenta: `LocalSystem`
- Además de reiniciar en crash / cambio de `credentials.json`, aplica
  `updates\pending-apply.json`: stop hijo → verifica SHA-256 de
  `updates\staging\app.mjs` → `.prev` → staging a `resources\agent\app.mjs`
  → spawn.
- **Permisos:** `ProgramData\InnoApp Agent` es modificable por `Users` (la
  bandeja escribe `credentials.json`), pero al arrancar el servicio deja
  `updates\` solo para SYSTEM y Administradores (`icacls`, sin herencia) y
  borra `pending-apply.json`, `staging\` y `app.mjs.prev` que hubiera. Si no
  puede, **OTA queda deshabilitado** (se loguea en `service-error.log`). El
  servicio ignora el `stagedPath` del marker: solo instala
  `updates\staging\app.mjs` y solo si su hash coincide con el `sha256` del marker.
- **Confirmación de arranque:** tras el swap queda `boot-check.json`. El agente
  nuevo lo borra al lograr su **primer heartbeat exitoso** (no al arrancar). Si
  el hijo muere a cualquier hora con el marker presente, o sigue vivo sin
  confirmar tras 180 s (`OTA_BOOT_GRACE`), el servicio lo mata, restaura
  `.prev` y deja `rolled-back.json`.
- Pendiente: el `icacls /grant Users:M /T` del instalador re-concede Users
  sobre `updates\`; se corrige solo al próximo arranque del servicio, que
  vuelve a cerrar el directorio.
