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
  updates\                  # Fase 1: staging + app.mjs.prev
```

Se conserva en uninstall/reinstall a propósito (activación y cola).

## Servicio

- Nombre SCM: `InnoAppAgent`
- Display: `InnoApp Agent`
- Cuenta: `LocalSystem`
