#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  Torot Icon Generator
#  Converts torot-logo.png into all Tauri-required icon formats.
#
#  Requires: ImageMagick (convert) or libvips (vips)
#  Run from project root: ./scripts/gen-icons.sh
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SOURCE="${1:-assets/torot-logo.png}"
OUT="src-tauri/icons"

if [[ ! -f "$SOURCE" ]]; then
  echo "Source image not found: $SOURCE"
  echo "Usage: $0 <path-to-logo.png>"
  exit 1
fi

mkdir -p "$OUT"

if command -v convert &>/dev/null; then
  echo "Using ImageMagick..."

  for SIZE in 32 128 256 512; do
    convert "$SOURCE" \
      -resize "${SIZE}x${SIZE}" \
      -background none \
      "$OUT/${SIZE}x${SIZE}.png"
    echo "  Created ${SIZE}x${SIZE}.png"
  done

  convert "$SOURCE" -resize "256x256" "$OUT/128x128@2x.png"
  echo "  Created 128x128@2x.png"
