#!/usr/bin/env bash
# Collect flat release files (no nested zip/tar.gz updater bundles).
# Usage: package-release-assets.sh <rust-target-triple> <artifact-name-prefix> [--flat-names]
set -euo pipefail

target="${1:?target triple required}"
name="${2:?artifact name prefix required}"
flat_names=0
if [[ "${3:-}" == "--flat-names" ]]; then
  flat_names=1
fi
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
        if ((flat_names)); then
          cp "$f" "${out}/$(basename "$f")"
        else
          cp "$f" "${out}/${name}-$(basename "$f")"
        fi
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
      if ((flat_names)); then
        cp "$f" "${out}/$(basename "$f")"
      else
        cp "$f" "${out}/${name}-$(basename "$f")"
      fi
    done
}

copy_docs() {
  local guide=""
  case "$name" in
    windows-*)
      if ((flat_names)); then
        guide="packaging/release-assets-readme-windows.md"
      else
        guide="packaging/release-user-guide-windows.md"
      fi
      ;;
    macos-*)
      if ((flat_names)); then
        guide="packaging/release-assets-readme-macos.md"
      else
        guide="packaging/release-user-guide-macos.md"
      fi
      ;;
  esac
  if [[ -n "$guide" && -f "$guide" ]]; then
    if ((flat_names)); then
      cp "$guide" "${out}/README.md"
    else
      cp "$guide" "${out}/${name}-README.md"
    fi
  fi
  if [[ -f ".env.example" ]]; then
    if ((flat_names)); then
      cp ".env.example" "${out}/.env.example"
    else
      cp ".env.example" "${out}/${name}-env.example"
    fi
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
