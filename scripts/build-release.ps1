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

$Target = "x86_64-pc-windows-msvc"
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

Write-Host "[1/4] Frontend (npm ci, icons, vite build)..." -ForegroundColor Yellow
Push-Location (Join-Path $Root "apps\themis-tray")
npm ci
npm run icons
npm run build
Pop-Location

Write-Host "[2/4] Rust release (themis-service, themis-cli, themis-tray)..." -ForegroundColor Yellow
cargo build --release -p themis-service -p themis-cli -p themis-tray --target $Target

if (-not $SkipInstaller) {
    Write-Host "[3/4] Tauri bundle (NSIS installer)..." -ForegroundColor Yellow
    Push-Location (Join-Path $Root "apps\themis-tray")
    $env:CI = "true"
    $tauriConfig = '{"build":{"beforeBuildCommand":""}}'
    npm run tauri build -- --target $Target --bundles $Bundle --config $tauriConfig
    Pop-Location
} else {
    Write-Host "[3/4] Skipped Tauri bundle (-SkipInstaller)." -ForegroundColor DarkYellow
}

Write-Host "[4/4] Collect release assets..." -ForegroundColor Yellow
& (Join-Path $Root "scripts\package-release-assets.ps1") -Target $Target -Name $Name
Copy-Item (Join-Path $Root "packaging\RELEASE-INDEX.md") (Join-Path $OutDir "README.md") -Force

Write-Host ""
Write-Host "Done. Release files:" -ForegroundColor Green
Get-ChildItem $OutDir | Format-Table Name, Length -AutoSize
Write-Host ""
Write-Host "Next: zip or upload everything in:" -ForegroundColor Cyan
Write-Host "  $OutDir"
Write-Host ""
Write-Host "Manual GitHub Release: gh release create vX.Y.Z $OutDir\*" -ForegroundColor DarkGray
Write-Host ""
