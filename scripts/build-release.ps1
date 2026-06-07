# One-shot local Release build (same steps as .github/workflows/release.yml on Windows).
# Usage:
#   .\scripts\build-release.ps1
#   .\scripts\build-release.cmd
#   .\scripts\build-release.ps1 -SkipInstaller   # exe only, no NSIS (faster)
param(
    [switch]$SkipInstaller
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

# rustup/cargo are often missing from non-interactive PowerShell PATH
$CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $CargoBin) {
    $env:Path = "$CargoBin;$env:Path"
}

function Invoke-Checked {
    param([scriptblock]$Command, [string]$Step)
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $Command
        $exit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
    }
    if ($exit -ne 0) {
        throw "$Step failed (exit $exit)."
    }
}

function Ensure-RustTarget {
    param([Parameter(Mandatory = $true)][string]$Triple)
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        Write-Warning "rustup not in PATH; if build fails with E0463, run: rustup target add $Triple"
        return
    }
    $installed = rustup target list --installed
    if ($installed -contains $Triple) { return }
    Write-Host "Installing Rust target $Triple (required for release build)..." -ForegroundColor Yellow
    Invoke-Checked { rustup target add $Triple } "rustup target add $Triple"
}

function Stop-ThemisProcesses {
    foreach ($procName in @("themis-tray", "themis-service")) {
        Get-Process -Name $procName -ErrorAction SilentlyContinue |
            Stop-Process -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 800
}

function Remove-TauriCliNativeModules {
    param([string]$TrayDir)
    $patterns = @(
        "node_modules\@tauri-apps\cli-win32-x64-msvc",
        "node_modules\@tauri-apps\cli-win32-arm64-msvc",
        "node_modules\@tauri-apps\cli-win32-ia32-msvc"
    )
    foreach ($rel in $patterns) {
        $path = Join-Path $TrayDir $rel
        if (Test-Path $path) {
            Remove-Item $path -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Windows EPERM on npm ci: Tauri CLI .node is locked while tray/tauri dev is running.
function Invoke-NpmCiTray {
    param([string]$TrayDir)
    Push-Location $TrayDir
    Stop-ThemisProcesses

    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $maxAttempts = 3
        for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
            $output = & npm ci 2>&1 | Out-String
            $exit = $LASTEXITCODE
            if ($exit -eq 0) { return }

            Write-Host $output
            $eperm = $output -match "EPERM|EBUSY|operation not permitted|-4048"
            if (-not $eperm) {
                throw "npm ci failed (exit $exit)."
            }
            if ($attempt -eq $maxAttempts) {
                Write-Host "npm ci still EPERM; falling back to npm install..." -ForegroundColor Yellow
                $output = & npm install 2>&1 | Out-String
                if ($LASTEXITCODE -ne 0) {
                    Write-Host $output
                    throw "npm install failed (exit $LASTEXITCODE). Close themis-tray / tauri dev and retry."
                }
                return
            }
            Write-Host "npm ci locked files (attempt $attempt/$maxAttempts); retrying after cleanup..." -ForegroundColor Yellow
            Stop-ThemisProcesses
            Remove-TauriCliNativeModules -TrayDir $TrayDir
            Start-Sleep -Seconds 2
        }
    } finally {
        $ErrorActionPreference = $prevEap
        Pop-Location
    }
}

$Target = "x86_64-pc-windows-msvc"
Ensure-RustTarget $Target
$Name = "windows-x86_64"
$Bundle = "nsis"
$OutDir = Join-Path (Join-Path $Root "release-assets") $Name

$env:CARGO_TERM_COLOR = "always"
$env:THEMIS_USE_MOCK_SPEECH = "true"
$env:CARGO_PROFILE_RELEASE_LTO = "thin"
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"
# CI uses lld-link for faster links; omit locally if you do not have LLVM lld installed.
if ($env:THEMIS_USE_LLD -eq "1") {
    $env:RUSTFLAGS = "-C link-arg=-fuse-ld=lld-link"
}

Write-Host ""
Write-Host "=== Themis local Release build ($Name) ===" -ForegroundColor Cyan
Write-Host "Output: $OutDir"
Write-Host ""

$TrayDir = Join-Path $Root "apps\themis-tray"
Write-Host "[1/4] Frontend (npm ci, icons, vite build)..." -ForegroundColor Yellow
Write-Host "  (stopping themis-tray/service if running — avoids npm EPERM on Tauri CLI)" -ForegroundColor DarkGray
Invoke-NpmCiTray -TrayDir $TrayDir
Push-Location $TrayDir
Invoke-Checked { npm run icons } "npm run icons"
Invoke-Checked { npm run build } "npm run build"
Pop-Location

Write-Host "[2/4] Rust release (themis-service, themis-cli)..." -ForegroundColor Yellow
Invoke-Checked {
    cargo build --release -p themis-service -p themis-cli --target $Target
} "cargo build --release (service, cli)"

# themis-tray MUST go through `tauri build` so dist/ UI is embedded (cargo-only = stale overlay).
Write-Host "[3/4] Tauri app (themis-tray, embed frontend)..." -ForegroundColor Yellow
Stop-ThemisProcesses
Push-Location $TrayDir
$env:CI = "true"
Invoke-Checked { cargo clean -p themis-tray --target $Target } "cargo clean -p themis-tray"
$TauriConfigFile = (Resolve-Path (Join-Path $Root "scripts\tauri-release-build.json")).Path
function Invoke-TauriBuild {
    param([string[]]$ExtraArgs, [string]$Label)
    $npmArgs = @(
        "run", "tauri", "build", "--",
        "--target", $Target,
        "--config", $TauriConfigFile
    ) + $ExtraArgs
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & npm @npmArgs
        $exit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
    }
    if ($exit -ne 0) { throw "$Label failed (exit $exit)." }
}
if ($SkipInstaller) {
    Write-Host "  (-SkipInstaller: no NSIS installer, still building tray exe)" -ForegroundColor DarkGray
    Invoke-TauriBuild -ExtraArgs @("--no-bundle") -Label "npm run tauri build --no-bundle"
} else {
    Invoke-TauriBuild -ExtraArgs @("--bundles", $Bundle) -Label "npm run tauri build"
}
Pop-Location

Write-Host "[4/4] Collect release assets..." -ForegroundColor Yellow
& (Join-Path $Root "scripts\package-release-assets.ps1") -Target $Target -Name $Name -FlatNames
$packedTray = Join-Path $OutDir "themis-tray.exe"
$builtTray = Join-Path $Root "target\$Target\release\themis-tray.exe"
if ((Get-Item $packedTray).Length -ne (Get-Item $builtTray).Length) {
    throw "release themis-tray.exe does not match target/$Target build (stale copy?)."
}

Remove-Item Env:THEMIS_USE_MOCK_SPEECH -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Done. Portable files: $OutDir" -ForegroundColor Green
$zipPath = Join-Path "release-assets" "Themis-$Name.zip"
Write-Host "Release zip (upload this): $zipPath" -ForegroundColor Green
Get-ChildItem $OutDir | Format-Table Name, Length -AutoSize
if (Test-Path $zipPath) {
    Get-ChildItem $zipPath | Format-Table Name, Length -AutoSize
}
Write-Host ""
Write-Host "Next: tag and push to trigger CI, or:" -ForegroundColor Cyan
Write-Host "  gh release create vX.Y.Z $zipPath packaging/RELEASE-INDEX.md"
Write-Host ""
Write-Host "Run tray (use a new terminal, or after build script exits):" -ForegroundColor Cyan
Write-Host "  cd $OutDir"
Write-Host "  # optional: copy .env.example .env — or use tray 配置 button after start"
Write-Host "  taskkill /IM themis-service.exe /F 2>`$null; .\themis-tray.exe"
Write-Host ""
