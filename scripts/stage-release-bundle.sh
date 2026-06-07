#!/usr/bin/env bash
# Patch Tauri .app / NSIS staging so release matches dev: fresh tray + bundled service.
# Usage: stage-release-bundle.sh <rust-target-triple> <release-assets-out-dir>
set -euo pipefail

target="${1:?target triple required}"
out="${2:?output dir required}"

release_base="target/${target}/release"
if [[ ! -d "$release_base" ]]; then
  release_base="target/release"
fi
if [[ ! -d "$release_base" ]]; then
  echo "stage-release-bundle: no release dir for ${target}" >&2
  exit 1
fi

copy_sidecars_into_app() {
  local app="$1"
  local macos="$app/Contents/MacOS"
  [[ -d "$macos" ]] || return 1
  for bin in themis-service themis-cli; do
    if [[ -f "${release_base}/${bin}" ]]; then
      cp "${release_base}/${bin}" "${macos}/${bin}"
      chmod +x "${macos}/${bin}"
      echo "staged ${bin} -> ${macos}/"
    fi
  done
  return 0
}

copy_tray_from_app() {
  local app="$1"
  local macos="$app/Contents/MacOS"
  local tray="${macos}/themis-tray"
  if [[ ! -f "$tray" ]]; then
    tray="$(find "$macos" -maxdepth 1 -type f -perm +111 ! -name "themis-service" ! -name "themis-cli" | head -1)"
  fi
  if [[ -f "$tray" ]]; then
    cp "$tray" "${out}/themis-tray"
    chmod +x "${out}/themis-tray"
    echo "staged themis-tray from ${app} ($(stat -f%z "$tray" 2>/dev/null || stat -c%s "$tray") bytes)"
    return 0
  fi
  return 1
}

rebuild_dmg_from_app() {
  local app="$1"
  command -v hdiutil >/dev/null 2>&1 || return 0
  local dmg_dir="${release_base}/bundle/dmg"
  local staging
  staging="$(mktemp -d)"
  mkdir -p "$dmg_dir"
  find "$dmg_dir" -maxdepth 1 -name "*.dmg" -delete 2>/dev/null || true

  cp -R "$app" "${staging}/"
  local install_txt
  install_txt="$(cd "$(dirname "$0")/.." && pwd)/packaging/macos-dmg-install.txt"
  if [[ -f "$install_txt" ]]; then
    cp "$install_txt" "${staging}/请先阅读-安装说明.txt"
  fi

  local arch dmg_path
  arch="$(uname -m)"
  dmg_path="${dmg_dir}/Themis_${arch}.dmg"
  hdiutil create -volname "Themis" -srcfolder "$staging" -ov -format UDZO "$dmg_path" >/dev/null
  rm -rf "$staging"
  echo "rebuilt ${dmg_path} (with install readme)"
}

sign_macos_release_app() {
  local app="$1"
  command -v codesign >/dev/null 2>&1 || return 0
  local macos="${app}/Contents/MacOS"
  for bin in themis-service themis-cli themis-tray; do
    if [[ -f "${macos}/${bin}" ]]; then
      codesign --force --sign - "${macos}/${bin}" 2>/dev/null || true
    fi
  done
  for bin in "${macos}"/*; do
    [[ -f "$bin" && -x "$bin" ]] || continue
    codesign --force --sign - "$bin" 2>/dev/null || true
  done
  if codesign --force --deep --sign - "$app" 2>/dev/null; then
    echo "ad-hoc signed ${app}"
  else
    echo "warn: codesign failed for ${app} (users may need: xattr -cr)" >&2
  fi
}

if [[ "$(uname -s)" == "Darwin" ]]; then
  app="$(find "${release_base}/bundle/macos" -maxdepth 1 -name "*.app" -type d 2>/dev/null | head -1 || true)"
  if [[ -n "$app" ]]; then
    copy_sidecars_into_app "$app"
    sign_macos_release_app "$app"
    copy_tray_from_app "$app" || true
    rebuild_dmg_from_app "$app"
  fi
fi

# Windows: copy service/cli next to the NSIS / target tray exe (portable folder layout).
if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || -n "${WINDIR:-}" ]]; then
  :
fi
