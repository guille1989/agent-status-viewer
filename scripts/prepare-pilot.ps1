$ErrorActionPreference = "Stop"

$viewerRoot = Split-Path -Parent $PSScriptRoot
$workspaceRoot = Split-Path -Parent $viewerRoot
$agentRoot = Join-Path $workspaceRoot "print-capture-agent"
$pipeRoot = Join-Path $workspaceRoot "print-capture-agent-pipe-server"
$serviceRoot = Join-Path $viewerRoot "agent-service"
$resourcesRoot = Join-Path $viewerRoot "src-tauri\resources"
$runtimeRoot = Join-Path $resourcesRoot "agent"

& npm.cmd --prefix $pipeRoot run build
if ($LASTEXITCODE -ne 0) { throw "Falló el build del pipe server" }
& npm.cmd --prefix $agentRoot run build
if ($LASTEXITCODE -ne 0) { throw "Falló el build del agente" }

New-Item -ItemType Directory -Path $runtimeRoot -Force | Out-Null
$esbuild = Join-Path $pipeRoot "node_modules\esbuild\bin\esbuild"
$outputFile = Join-Path $runtimeRoot "app.mjs"
$outputArgument = "--outfile=$outputFile"
& node $esbuild (Join-Path $agentRoot "src\index.ts") --bundle --platform=node --format=esm --external:serialport $outputArgument
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

Write-Output "Runtime piloto preparado en $runtimeRoot"
Write-Output "Servicio copiado a $resourcesRoot\innoapp-agent-service.exe"
