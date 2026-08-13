$ErrorActionPreference = "Stop"

$viewerRoot = Split-Path -Parent $PSScriptRoot
$workspaceRoot = Split-Path -Parent $viewerRoot
$agentRoot = Join-Path $workspaceRoot "print-capture-agent"
$pipeRoot = Join-Path $workspaceRoot "print-capture-agent-pipe-server"
$runtimeRoot = Join-Path $viewerRoot "src-tauri\resources\agent"

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

Write-Output "Runtime piloto preparado en $runtimeRoot"
