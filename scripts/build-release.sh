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

echo "[2/4] Rust release (themis-service, themis-cli)..."
cargo build --release -p themis-service -p themis-cli --target "$TARGET"

echo "[3/4] Tauri app (themis-tray, embed frontend)..."
pkill -x themis-tray 2>/dev/null || true
pkill -x themis-service 2>/dev/null || true
sleep 0.5
pushd apps/themis-tray >/dev/null
export CI=true
cargo clean -p themis-tray --target "$TARGET"
tauri_cfg="$ROOT/scripts/tauri-release-build.json"
if [[ "$SKIP_INSTALLER" -eq 1 ]]; then
  echo "  (--skip-installer: no dmg, still building tray)"
  npm run tauri build -- --target "$TARGET" --no-bundle --config "$tauri_cfg"
else
  npm run tauri build -- --target "$TARGET" --bundles dmg --config "$tauri_cfg"
fi
popd >/dev/null

echo "[4/4] Collect release assets..."
bash scripts/package-release-assets.sh "$TARGET" "$NAME" --flat-names

unset THEMIS_USE_MOCK_SPEECH || true

echo ""
echo "Done. Portable files: $OUT_DIR"
echo "Release zip (upload this): release-assets/Themis-${NAME}.zip"
ls -la "$OUT_DIR"
ls -la "release-assets/Themis-${NAME}.zip"
echo ""
echo "Next: tag and push to trigger CI, or:"
echo "  gh release create vX.Y.Z release-assets/Themis-${NAME}.zip packaging/RELEASE-INDEX.md"
echo ""
