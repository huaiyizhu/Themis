#!/usr/bin/env bash
# Themis development helper (macOS / Linux)
# Usage: ./scripts/themis.sh [build|start|stop|restart|dev|tray|all|status|doctor] [-r|--release]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RELEASE=0
CMD="${1:-dev}"

if [[ "${2:-}" == "-r" || "${2:-}" == "--release" ]]; then
  RELEASE=1
fi
if [[ "$CMD" == "-r" || "$CMD" == "--release" ]]; then
  RELEASE=1
  CMD="${2:-dev}"
fi

profile() { [[ "$RELEASE" -eq 1 ]] && echo release || echo debug; }
service_bin() { echo "$ROOT/target/$(profile)/themis-service"; }

ensure_env() {
  if [[ ! -f "$ROOT/.env" && -f "$ROOT/.env.example" ]]; then
    cp "$ROOT/.env.example" "$ROOT/.env"
    echo "Created .env from .env.example"
  fi
}

build_service() {
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
  echo "Next: ./scripts/themis.sh tray"
}

start_tray() {
  ensure_env
  build_service
  start_service
  local tray="$ROOT/apps/themis-tray"
  if [[ ! -d "$tray/node_modules" ]]; then
    (cd "$tray" && npm install)
  fi
  echo "Starting Tauri tray. Hotkey: Cmd+Shift+T"
  (cd "$tray" && npm run tauri dev)
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
  doctor) ensure_env; cargo run -p themis-cli -- doctor ;;
  *)
    echo "Usage: $0 [build|start|stop|restart|dev|tray|all|status|doctor] [-r|--release]"
    exit 1
    ;;
esac
