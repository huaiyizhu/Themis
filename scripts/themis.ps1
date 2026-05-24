# Themis development helper (Windows PowerShell)
# Usage:
#   .\scripts\themis.ps1 dev       # build + start themis-service (background)
#   .\scripts\themis.ps1 restart   # stop, rebuild, start service
#   .\scripts\themis.ps1 tray       # build + start service + Tauri tray (foreground)
#   .\scripts\themis.ps1 build     # build themis-service only
#   .\scripts\themis.ps1 start       # start service (build if binary missing)
#   .\scripts\themis.ps1 stop      # stop running themis-service
#   .\scripts\themis.ps1 status    # show process + .env state
#   .\scripts\themis.ps1 doctor    # run themis-cli doctor
#
# Add -Release for release profile binaries.

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('build', 'start', 'stop', 'restart', 'dev', 'tray', 'all', 'status', 'doctor', 'probe')]
    [string]$Command = 'dev',

    [switch]$Release
)

$ErrorActionPreference = 'Stop'

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root

function Get-ProfileName {
    if ($Release) { 'release' } else { 'debug' }
}

function Get-ServiceExe {
    Join-Path $Root "target\$(Get-ProfileName)\themis-service.exe"
}

function Ensure-EnvFile {
    $envFile = Join-Path $Root '.env'
    $example = Join-Path $Root '.env.example'
    if (-not (Test-Path $envFile)) {
        if (Test-Path $example) {
            Copy-Item $example $envFile
            Write-Host "Created .env from .env.example — edit AZURE_SPEECH_KEY before real transcription."
        } else {
            Write-Warning ".env not found. Copy .env.example to .env and set Azure Speech keys."
        }
    }
}

function Build-Service {
    Write-Host "Building themis-service ($(Get-ProfileName))..."
    $cargoArgs = @('build', '-p', 'themis-service')
    if ($Release) { $cargoArgs += '--release' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "Built: $(Get-ServiceExe)"
}

function Stop-ServiceProcess {
    $procs = Get-Process -Name 'themis-service' -ErrorAction SilentlyContinue
    if ($procs) {
        $procs | Stop-Process -Force
        Write-Host "Stopped themis-service."
        Start-Sleep -Milliseconds 400
    } else {
        Write-Host "themis-service is not running."
    }
}

function Start-ServiceProcess {
    Ensure-EnvFile
    $exe = Get-ServiceExe
    if (-not (Test-Path $exe)) {
        Write-Host "Binary missing, building first..."
        Build-Service
    }

    Stop-ServiceProcess

    # Console subsystem exe — use CreateNoWindow so dev does not pop a blank terminal.
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.WorkingDirectory = $Root
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $proc = [System.Diagnostics.Process]::Start($psi)
    Start-Sleep -Seconds 1

    if ($proc.HasExited) {
        Write-Error "themis-service exited immediately (code $($proc.ExitCode)). Check logs in %LOCALAPPDATA%\Themis\logs"
    }

    $port = '50051'
    if (Test-Path (Join-Path $Root '.env')) {
        $line = Get-Content (Join-Path $Root '.env') | Where-Object { $_ -match '^\s*THEMIS_GRPC_PORT\s*=' } | Select-Object -First 1
        if ($line -match '=\s*(\d+)') { $port = $Matches[1] }
    }

    Write-Host ""
    Write-Host "themis-service running"
    Write-Host "  PID:     $($proc.Id)"
    Write-Host "  gRPC:    127.0.0.1:$port"
    Write-Host "  logs:    $env:LOCALAPPDATA\Themis\logs"
    Write-Host ""
    Write-Host "Next: .\scripts\themis.ps1 tray   (or Ctrl+Shift+T in the tray app)"
}

function Start-Tray {
    Ensure-EnvFile
    Build-Service
    Start-ServiceProcess

    $trayDir = Join-Path $Root 'apps\themis-tray'
    if (-not (Test-Path (Join-Path $trayDir 'node_modules'))) {
        Write-Host "Installing tray dependencies (npm install)..."
        Push-Location $trayDir
        npm install
        Pop-Location
    }

    Write-Host "Starting Tauri tray (foreground). Close the window to exit the UI; service keeps running."
    Write-Host "Hotkey: Ctrl+Shift+T — toggle capture"
    Push-Location $trayDir
    npm run tauri dev
    Pop-Location
}

function Show-Status {
    Ensure-EnvFile
    $procs = Get-Process -Name 'themis-service' -ErrorAction SilentlyContinue
    if ($procs) {
        foreach ($p in $procs) {
            Write-Host "themis-service: running (PID $($p.Id))"
        }
    } else {
        Write-Host "themis-service: not running"
    }

    $exe = Get-ServiceExe
    if (Test-Path $exe) {
        Write-Host "binary: $exe"
    } else {
        Write-Host "binary: not built — run .\scripts\themis.ps1 build"
    }

    if (Test-Path (Join-Path $Root '.env')) {
        Write-Host ".env: present"
    } else {
        Write-Host ".env: MISSING"
    }
}

switch ($Command) {
    'build'  { Build-Service }
    'start'  { Start-ServiceProcess }
    'stop'   { Stop-ServiceProcess }
    'restart' {
        Stop-ServiceProcess
        Build-Service
        Start-ServiceProcess
    }
    'dev' {
        Build-Service
        Start-ServiceProcess
    }
    'tray' { Start-Tray }
    'all'  { Start-Tray }
    'status' { Show-Status }
    'doctor' {
        Ensure-EnvFile
        & cargo run -p themis-cli -- doctor
    }
    'probe' {
        Ensure-EnvFile
        Write-Host "Play audio (e.g. YouTube), then keep this running..."
        & cargo run -p themis-cli -- audio-probe --seconds 8
    }
    default { Write-Error "Unknown command: $Command" }
}
