#!/usr/bin/env bash
# Fix macOS Gatekeeper "已损坏，无法打开" for Themis.app from GitHub Releases.
# Usage:
#   ./scripts/fix-macos-app-quarantine.sh
#   ./scripts/fix-macos-app-quarantine.sh /Applications/Themis.app
set -euo pipefail

APP="${1:-/Applications/Themis.app}"

if [[ ! -d "$APP" ]]; then
  echo "error: app not found: $APP" >&2
  echo "usage: $0 [/path/to/Themis.app]" >&2
  exit 1
fi

echo "Removing quarantine from: $APP"
xattr -cr "$APP"

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP" 2>/dev/null || true
fi

echo "Done. Launch with: open \"$APP\""
