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
src-tauri/target/release/bundle/nsis/InnoApp Agent_0.1.0_x64-setup.exe
```

Durante el piloto la aplicación solicita el código de activación, guarda la
credencial en el directorio de datos de la aplicación y mantiene el agente
como proceso hijo mientras la aplicación de bandeja está abierta. La captura
permanece desactivada: esta versión valida instalación, activación, detección
de puertos y heartbeat antes de convertir el proceso en un servicio Windows.

El script reproducible que prepara el runtime es
[`scripts/prepare-pilot.ps1`](scripts/prepare-pilot.ps1).
