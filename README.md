# InnoApp Agent

Aplicación de bandeja Tauri que muestra el estado del agente de captura y se
comunica con él mediante el named pipe `\\.\pipe\print-capture-agent`.

## Desarrollo

```powershell
npm install
npm run tauri dev
```

## Instalador piloto

La distribución piloto empaqueta `print-capture-agent`, un runtime Node y las
dependencias nativas de `serialport`. El usuario final no necesita instalar
Node.js.

```powershell
npm run build:pilot
```

El instalador queda en:

```text
src-tauri/target/release/bundle/nsis/InnoApp Agent_0.2.0_x64-setup.exe
```

Durante el piloto la aplicación solicita el código de activación, guarda la
credencial en el directorio de datos de la aplicación y mantiene el agente
como proceso hijo mientras la aplicación de bandeja está abierta.

Desde la versión `0.2.0` la **captura viene prendida**: ya se validaron
instalación, activación, detección de puertos y heartbeat contra PCs reales,
así que el agente abre los puertos serie detectados (y conecta los periféricos
TCP configurados) para leer tickets. Para dejar una instalación en modo "solo
diagnóstico" sin recompilar, definir `ENABLE_CAPTURE=false` en el entorno del
sistema antes de abrir la aplicación. Convertir el proceso en un servicio
Windows sigue pendiente (hoy corre como hijo de la app de bandeja).

El script reproducible que prepara el runtime es
[`scripts/prepare-pilot.ps1`](scripts/prepare-pilot.ps1).
