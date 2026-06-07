# Collect flat release files (no nested zip/tar.gz updater bundles).
param(
    [Parameter(Mandatory = $true)][string]$Target,
    [Parameter(Mandatory = $true)][string]$Name,
    # Local build-release: files already live under release-assets/<Name>/ — omit Name- prefix.
    [switch]$FlatNames
)

$ErrorActionPreference = "Stop"
$out = Join-Path "release-assets" $Name
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Copy-WithPrefix {
    param([string]$SourcePath)
    if (-not (Test-Path $SourcePath -PathType Leaf)) { return }
    $leaf = Split-Path $SourcePath -Leaf
    $destName = if ($FlatNames) { $leaf } else { "$Name-$leaf" }
    Copy-Item $SourcePath (Join-Path $out $destName) -Force
}

function Copy-Docs {
    $guide = switch -Wildcard ($Name) {
        "windows-*" {
            if ($FlatNames) { "packaging/release-assets-readme-windows.md" }
            else { "packaging/release-user-guide-windows.md" }
        }
        "macos-*" {
            if ($FlatNames) { "packaging/release-assets-readme-macos.md" }
            else { "packaging/release-user-guide-macos.md" }
        }
        default { $null }
    }
    if ($guide -and (Test-Path $guide)) {
        $readmeName = if ($FlatNames) { "README.md" } else { "$Name-README.md" }
        Copy-Item $guide (Join-Path $out $readmeName) -Force
    }
    if (Test-Path ".env.example") {
        $envName = if ($FlatNames) { ".env.example" } else { "$Name-env.example" }
        Copy-Item ".env.example" (Join-Path $out $envName) -Force
    }
}

# Use only the requested target triple. Do NOT also copy target/release — on Windows ARM
# dev machines that folder holds stale host (aarch64) binaries and overwrites the x64 build.
$releaseBase = "target/$Target/release"
if (-not (Test-Path $releaseBase)) {
    $releaseBase = "target/release"
}
if (-not (Test-Path $releaseBase)) {
    throw "No release binaries under target/$Target/release or target/release"
}
Write-Host "Collecting binaries from: $releaseBase"
foreach ($pattern in @("themis-service*", "themis-cli*", "themis-tray*")) {
    Get-ChildItem $releaseBase -Filter $pattern -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -ne ".d" } |
        ForEach-Object {
        Copy-WithPrefix $_.FullName
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

function Get-PlatformFolderName {
    switch -Wildcard ($Name) {
        "windows-*" { return "Themis-Windows" }
        "macos-aarch64" { return "Themis-macOS-Apple-Silicon" }
        "macos-x86_64" { return "Themis-macOS-Intel" }
        default { return "Themis-$Name" }
    }
}

function New-PlatformZip {
    $folder = Get-PlatformFolderName
    $zipPath = Join-Path "release-assets" "Themis-$Name.zip"
    $staging = Join-Path ([System.IO.Path]::GetTempPath()) ("themis-zip-$Name-" + [guid]::NewGuid().ToString())
    $bundleDir = Join-Path $staging $folder
    New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null
    Copy-Item (Join-Path $out "*") $bundleDir -Recurse -Force
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    Compress-Archive -Path $bundleDir -DestinationPath $zipPath -Force
    Remove-Item $staging -Recurse -Force
    Write-Host "Platform zip: $zipPath  ->  $folder/"
}

$files = Get-ChildItem $out -File
if ($files.Count -eq 0) {
    throw "No release assets collected under $out"
}

New-PlatformZip

Write-Host "Release assets ($($files.Count) files in $out):"
Get-ChildItem $out | Format-Table Name, Length
Get-ChildItem (Join-Path "release-assets" "Themis-$Name.zip") | Format-Table Name, Length
