#!/usr/bin/env bash
# Collect flat release files (no nested zip/tar.gz updater bundles).
# Usage: package-release-assets.sh <rust-target-triple> <artifact-name-prefix>
set -euo pipefail

target="${1:?target triple required}"
name="${2:?artifact name prefix required}"
out="release-assets/${name}"
mkdir -p "$out"

copy_binaries() {
  shopt -s nullglob
  for base in "target/${target}/release" "target/release"; do
    [[ -d "$base" ]] || continue
    for pattern in themis-service themis-cli themis-tray; do
      for f in "${base}/${pattern}"*; do
        [[ -f "$f" ]] || continue
        case "$f" in *.d) continue ;; esac
        cp "$f" "${out}/${name}-$(basename "$f")"
      done
    done
  done
}

find_bundle_dir() {
  for b in \
    "target/${target}/release/bundle" \
    "target/release/bundle" \
    "apps/themis-tray/src-tauri/target/${target}/release/bundle"; do
    if [[ -d "$b" ]]; then
      echo "$b"
      return 0
    fi
  done
  return 1
}

copy_installers() {
  local bundle="$1"
  find "$bundle" -type f \( -name '*.msi' -o -name '*.dmg' -o -name '*-setup.exe' \) \
    ! -name '*.zip' \
    ! -name '*.tar.gz' \
    ! -name '*.sig' \
    -print0 | while IFS= read -r -d '' f; do
      cp "$f" "${out}/${name}-$(basename "$f")"
    done
}

copy_docs() {
  local guide=""
  case "$name" in
    windows-*) guide="packaging/release-user-guide-windows.md" ;;
    macos-*) guide="packaging/release-user-guide-macos.md" ;;
  esac
  if [[ -n "$guide" && -f "$guide" ]]; then
    cp "$guide" "${out}/${name}-README.md"
  fi
  if [[ -f ".env.example" ]]; then
    cp ".env.example" "${out}/${name}-env.example"
  fi
}

copy_binaries
copy_docs
if bundle="$(find_bundle_dir)"; then
  copy_installers "$bundle"
fi

shopt -s nullglob
files=("$out"/*)
if ((${#files[@]} == 0)); then
  echo "No release assets collected under ${out}" >&2
  exit 1
fi

echo "Release assets (${#files[@]} files):"
ls -la "$out"
