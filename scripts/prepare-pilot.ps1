$ErrorActionPreference = "Stop"

$viewerRoot = Split-Path -Parent $PSScriptRoot
$workspaceRoot = Split-Path -Parent $viewerRoot
$agentRoot = Join-Path $workspaceRoot "print-capture-agent"
$pipeRoot = Join-Path $workspaceRoot "print-capture-agent-pipe-server"
$serviceRoot = Join-Path $viewerRoot "agent-service"
$resourcesRoot = Join-Path $viewerRoot "src-tauri\resources"
$runtimeRoot = Join-Path $resourcesRoot "agent"

function Read-JsonVersion([string]$path) {
  if (-not (Test-Path $path)) { throw "No existe $path" }
  return (Get-Content -LiteralPath $path -Raw | ConvertFrom-Json).version
}

# ---- Alineación de versiones (Fase 0 OTA) --------------------------------
# Producto Windows = package.json del viewer + tauri.conf.json + Cargo tray.
# Agent JS = package.json del print-capture-agent (fuente de verdad del
# heartbeat). Deben coincidir en un release de setup para no mentir al negocio.
$productVersion = Read-JsonVersion (Join-Path $viewerRoot "package.json")
$tauriVersion = Read-JsonVersion (Join-Path $viewerRoot "src-tauri\tauri.conf.json")
$agentVersion = Read-JsonVersion (Join-Path $agentRoot "package.json")
$trayCargo = Get-Content -LiteralPath (Join-Path $viewerRoot "src-tauri\Cargo.toml") -Raw
if ($tauriVersion -ne $productVersion) {
  throw "Versión desalineada: package.json=$productVersion vs tauri.conf.json=$tauriVersion"
}
if ($trayCargo -notmatch "(?m)^version\s*=\s*`"$([regex]::Escape($productVersion))`"") {
  throw "Versión desalineada: src-tauri/Cargo.toml no tiene version = `"$productVersion`""
}
if ($agentVersion -ne $productVersion) {
  throw "Versión desalineada: producto=$productVersion vs print-capture-agent=$agentVersion. Bumpear ambos juntos antes del setup."
}
$serviceCargo = Get-Content -LiteralPath (Join-Path $serviceRoot "Cargo.toml") -Raw
if ($serviceCargo -match '(?m)^version\s*=\s*"([^"]+)"') {
  Write-Output "Servicio Rust version=$($Matches[1]) (independiente del producto; documentar si este release lo exige bump)."
}

& npm.cmd --prefix $pipeRoot run build
if ($LASTEXITCODE -ne 0) { throw "Falló el build del pipe server" }
& npm.cmd --prefix $agentRoot run build
if ($LASTEXITCODE -ne 0) { throw "Falló el build del agente" }

New-Item -ItemType Directory -Path $runtimeRoot -Force | Out-Null
$esbuild = Join-Path $pipeRoot "node_modules\esbuild\bin\esbuild"
$outputFile = Join-Path $runtimeRoot "app.mjs"
$outputArgument = "--outfile=$outputFile"
# Inyecta la versión del package.json del agent para que el heartbeat no
# dependa de un package.json en Program Files.
# PowerShell strips bare quotes from external args; keep \" so esbuild sees a JS string.
$defineVersion = '--define:process.env.AGENT_VERSION=\"' + $agentVersion + '\"'
& node $esbuild (Join-Path $agentRoot "src\index.ts") --bundle --platform=node --format=esm --external:serialport $outputArgument $defineVersion
if ($LASTEXITCODE -ne 0) { throw "Falló el bundle del agente" }

$nodeExe = (Get-Command node).Source
Copy-Item -LiteralPath $nodeExe -Destination (Join-Path $runtimeRoot "node.exe") -Force
& npm.cmd --prefix $runtimeRoot install --omit=dev --no-audit --no-fund
if ($LASTEXITCODE -ne 0) { throw "No se pudieron preparar las dependencias del runtime" }

# Servicio Windows (crate agent-service) que corre el agente como LocalSystem.
& cargo build --release --manifest-path (Join-Path $serviceRoot "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "Falló el build del servicio" }
Copy-Item -LiteralPath (Join-Path $serviceRoot "target\release\innoapp-agent-service.exe") `
          -Destination (Join-Path $resourcesRoot "innoapp-agent-service.exe") -Force

Write-Output "Runtime piloto preparado en $runtimeRoot (agent v$agentVersion, producto v$productVersion)"
Write-Output "Servicio copiado a $resourcesRoot\innoapp-agent-service.exe"
Write-Output "Setup esperado: InnoApp Agent_${productVersion}_x64-setup.exe"
