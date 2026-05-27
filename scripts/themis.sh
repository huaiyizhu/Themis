#!/usr/bin/env bash
# Themis development helper (macOS / Linux)
# Usage: ./scripts/themis.sh [build|start|stop|restart|dev|tray|all|status|doctor|probe] [-r|--release]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RELEASE=0
CMD="${1:-dev}"

for arg in "${@:2}"; do
  if [[ "$arg" == "-r" || "$arg" == "--release" ]]; then
    RELEASE=1
  fi
done

if [[ "$CMD" == "-r" || "$CMD" == "--release" ]]; then
  RELEASE=1
  CMD="${2:-dev}"
fi

profile() { [[ "$RELEASE" -eq 1 ]] && echo release || echo debug; }
service_bin() { echo "$ROOT/target/$(profile)/themis-service"; }

# rustup installs to ~/.cargo/bin; new shells need `source ~/.cargo/env`
ensure_cargo() {
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi
  echo "error: cargo not found (Rust toolchain required)." >&2
  echo "" >&2
  echo "Install Rust (pick one):" >&2
  echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
  echo "  source \"\$HOME/.cargo/env\"" >&2
  echo "  # or: brew install rust" >&2
  echo "" >&2
  echo "Then re-run: ./scripts/themis.sh $CMD" >&2
  exit 127
}

ensure_env() {
  if [[ ! -f "$ROOT/.env" && -f "$ROOT/.env.example" ]]; then
    cp "$ROOT/.env.example" "$ROOT/.env"
    echo "Created .env from .env.example — edit AZURE_SPEECH_KEY before live transcription."
  fi
}

ensure_macos_icons() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi
  local icns="$ROOT/apps/themis-tray/src-tauri/icons/icon.icns"
  if [[ ! -f "$icns" ]]; then
    echo "icon.icns missing — generating for Tauri..."
    bash "$ROOT/scripts/prepare-macos-icons.sh"
  fi
}

build_service() {
  ensure_cargo
  echo "Building themis-service ($(profile))..."
  if [[ "$RELEASE" -eq 1 ]]; then
    cargo build -p themis-service --release
  else
    cargo build -p themis-service
  fi
  echo "Built: $(service_bin)"
}

stop_service() {
  if pkill -x themis-service 2>/dev/null; then
    echo "Stopped themis-service."
    sleep 0.4
  else
    echo "themis-service is not running."
  fi
}

start_service() {
  ensure_env
  local bin
  bin="$(service_bin)"
  if [[ ! -x "$bin" ]]; then
    echo "Binary missing, building..."
    build_service
  fi
  stop_service
  "$bin" &
  sleep 1
  echo ""
  echo "themis-service running (background)"
  echo "  logs: ~/Library/Logs/Themis (macOS) or ~/.local/share/themis/logs"
  echo ""
  echo "Note: dev only starts the service (state=idle). Audio capture starts when:"
  echo "  1) ./scripts/themis.sh tray  — then press Cmd+Shift+T (macOS) or Ctrl+Shift+T (Windows)"
  echo "  2) ./scripts/themis.sh probe — 8s capture test (no tray)"
  echo ""
  if [[ "$(uname -s)" == "Darwin" ]]; then
    echo "macOS system audio: install BlackHole, set Output+Input to BlackHole,"
    echo "  or add THEMIS_AUDIO_INPUT_DEVICE=BlackHole to .env"
  fi
}

start_tray() {
  ensure_env
  ensure_macos_icons
  build_service
  start_service
  local tray="$ROOT/apps/themis-tray"
  if [[ ! -d "$tray/node_modules" ]]; then
    (cd "$tray" && npm install)
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    echo "Starting Tauri tray. Hotkey: Cmd+Shift+T"
  else
    echo "Starting Tauri tray."
  fi
  (cd "$tray" && npm run tauri dev)
}

run_probe() {
  ensure_cargo
  ensure_env
  echo "Audio probe (8s) — play sound now."
  if [[ "$(uname -s)" == "Darwin" ]]; then
    echo "Tip: route system audio via BlackHole (docs/platform-notes.md)."
  fi
  cargo run -p themis-cli -- audio-probe --seconds 8
}

status_cmd() {
  ensure_env
  if pgrep -x themis-service >/dev/null; then
    echo "themis-service: running (PID $(pgrep -x themis-service))"
  else
    echo "themis-service: not running"
  fi
  if [[ -x "$(service_bin)" ]]; then
    echo "binary: $(service_bin)"
  else
    echo "binary: not built"
  fi
  [[ -f "$ROOT/.env" ]] && echo ".env: present" || echo ".env: MISSING"
}

case "$CMD" in
  build) build_service ;;
  start) start_service ;;
  stop) stop_service ;;
  restart) stop_service; build_service; start_service ;;
  dev) build_service; start_service ;;
  tray|all) start_tray ;;
  status) status_cmd ;;
  doctor) ensure_cargo; ensure_env; cargo run -p themis-cli -- doctor ;;
  probe) run_probe ;;
  icons)
    if [[ "$(uname -s)" != "Darwin" ]]; then
      echo "icons: macOS only (iconutil)."
      exit 1
    fi
    bash "$ROOT/scripts/prepare-macos-icons.sh"
    ;;
  *)
    echo "Usage: $0 [build|start|stop|restart|dev|tray|all|status|doctor|probe|icons] [-r|--release]"
    exit 1
    ;;
esac
