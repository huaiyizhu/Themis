# Collect flat release files (no nested zip/tar.gz updater bundles).
param(
    [Parameter(Mandatory = $true)][string]$Target,
    [Parameter(Mandatory = $true)][string]$Name
)

$ErrorActionPreference = "Stop"
$out = Join-Path "release-assets" $Name
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Copy-WithPrefix {
    param([string]$SourcePath)
    if (-not (Test-Path $SourcePath -PathType Leaf)) { return }
    $destName = "$Name-$(Split-Path $SourcePath -Leaf)"
    Copy-Item $SourcePath (Join-Path $out $destName) -Force
}

function Copy-Docs {
    $guide = switch -Wildcard ($Name) {
        "windows-*" { "packaging/release-user-guide-windows.md" }
        "macos-*"   { "packaging/release-user-guide-macos.md" }
        default     { $null }
    }
    if ($guide -and (Test-Path $guide)) {
        Copy-Item $guide (Join-Path $out "$Name-README.md") -Force
    }
    if (Test-Path ".env.example") {
        Copy-Item ".env.example" (Join-Path $out "$Name-env.example") -Force
    }
}

foreach ($base in @("target/$Target/release", "target/release")) {
    if (-not (Test-Path $base)) { continue }
    foreach ($pattern in @("themis-service*", "themis-cli*", "themis-tray*")) {
        Get-ChildItem $base -Filter $pattern -File -ErrorAction SilentlyContinue | ForEach-Object {
            Copy-WithPrefix $_.FullName
        }
    }
}

$bundleCandidates = @(
    "target/$Target/release/bundle",
    "target/release/bundle",
    "apps/themis-tray/src-tauri/target/$Target/release/bundle"
)
$bundle = $bundleCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($bundle) {
    foreach ($pat in @("*.msi", "*.dmg", "*-setup.exe")) {
        Get-ChildItem $bundle -Filter $pat -File -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.Extension -ne ".sig" } |
            ForEach-Object { Copy-WithPrefix $_.FullName }
    }
}

Copy-Docs

$files = Get-ChildItem $out -File
if ($files.Count -eq 0) {
    throw "No release assets collected under $out"
}

Write-Host "Release assets ($($files.Count) files):"
Get-ChildItem $out | Format-Table Name, Length
