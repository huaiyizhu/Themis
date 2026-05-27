#!/usr/bin/env bash
# Generate icon.icns for Tauri macOS builds (run once per clone, or when PNG icons change).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON_DIR="$ROOT/apps/themis-tray/src-tauri/icons"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "prepare-macos-icons.sh requires macOS (iconutil)."
  exit 1
fi

for f in 32x32.png 128x128.png 256x256.png; do
  if [[ ! -f "$ICON_DIR/$f" ]]; then
    echo "Missing $ICON_DIR/$f"
    exit 1
  fi
done

mkdir -p "$ICON_DIR/icon.iconset"
cp "$ICON_DIR/32x32.png" "$ICON_DIR/icon.iconset/icon_16x16.png"
cp "$ICON_DIR/32x32.png" "$ICON_DIR/icon.iconset/icon_16x16@2x.png"
cp "$ICON_DIR/32x32.png" "$ICON_DIR/icon.iconset/icon_32x32.png"
cp "$ICON_DIR/32x32.png" "$ICON_DIR/icon.iconset/icon_32x32@2x.png"
cp "$ICON_DIR/128x128.png" "$ICON_DIR/icon.iconset/icon_128x128.png"
cp "$ICON_DIR/128x128.png" "$ICON_DIR/icon.iconset/icon_128x128@2x.png"
cp "$ICON_DIR/256x256.png" "$ICON_DIR/icon.iconset/icon_256x256.png"
cp "$ICON_DIR/256x256.png" "$ICON_DIR/icon.iconset/icon_512x512@2x.png"
iconutil -c icns "$ICON_DIR/icon.iconset" -o "$ICON_DIR/icon.icns"
rm -rf "$ICON_DIR/icon.iconset"
echo "Wrote $ICON_DIR/icon.icns"
