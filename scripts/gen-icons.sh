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
