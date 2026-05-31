#!/usr/bin/env bash
# One-shot local Release build (same steps as .github/workflows/release.yml on macOS).
# Usage:
#   ./scripts/build-release.sh
#   ./scripts/build-release.sh --skip-installer
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SKIP_INSTALLER=0
if [[ "${1:-}" == "--skip-installer" ]]; then
  SKIP_INSTALLER=1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "build-release.sh is for macOS only. On Windows use: .\\scripts\\build-release.ps1" >&2
  exit 1
fi

case "$(uname -m)" in
  arm64) TARGET="aarch64-apple-darwin"; NAME="macos-aarch64" ;;
  x86_64) TARGET="x86_64-apple-darwin"; NAME="macos-x86_64" ;;
  *)
    echo "Unsupported macOS arch: $(uname -m)" >&2
    exit 1
    ;;
esac

OUT_DIR="$ROOT/release-assets/$NAME"

if command -v rustup >/dev/null 2>&1; then
  echo "Ensuring Rust target ${TARGET}..."
  rustup target add "${TARGET}"
fi

export CARGO_TERM_COLOR=always
export THEMIS_USE_MOCK_SPEECH=true
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16

echo ""
echo "=== Themis local Release build ($NAME) ==="
echo "Output: $OUT_DIR"
echo ""

echo "[1/4] macOS icons..."
bash scripts/prepare-macos-icons.sh

echo "[1/4] Frontend (npm ci, icons, vite build)..."
pushd apps/themis-tray >/dev/null
npm ci
npm run icons
npm run build
popd >/dev/null

echo "[2/4] Rust release (themis-service, themis-cli, themis-tray)..."
cargo build --release -p themis-service -p themis-cli -p themis-tray --target "$TARGET"

if [[ "$SKIP_INSTALLER" -eq 0 ]]; then
  echo "[3/4] Tauri bundle (dmg)..."
  pushd apps/themis-tray >/dev/null
  export CI=true
  npm run tauri build -- --target "$TARGET" --bundles dmg --config '{"build":{"beforeBuildCommand":""}}'
  popd >/dev/null
else
  echo "[3/4] Skipped Tauri bundle (--skip-installer)."
fi

echo "[4/4] Collect release assets..."
bash scripts/package-release-assets.sh "$TARGET" "$NAME" --flat-names

unset THEMIS_USE_MOCK_SPEECH || true

echo ""
echo "Done. Release files:"
ls -la "$OUT_DIR"
echo ""
echo "Next: upload everything in $OUT_DIR to GitHub Release"
echo "  gh release create vX.Y.Z $OUT_DIR/*"
echo ""
