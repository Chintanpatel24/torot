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

  # PNG sizes required by Tauri
  for SIZE in 32 128 256 512; do
    convert "$SOURCE" \
      -resize "${SIZE}x${SIZE}" \
      -background none \
      "$OUT/${SIZE}x${SIZE}.png"
    echo "  Created ${SIZE}x${SIZE}.png"
  done

  # @2x variant
  convert "$SOURCE" -resize "256x256" "$OUT/128x128@2x.png"
  echo "  Created 128x128@2x.png"

 # .icns for macOS — requires png2icns or sips
  if command -v png2icns &>/dev/null; then
    png2icns "$OUT/icon.icns" "$OUT/512x512.png" "$OUT/256x256.png" \
      "$OUT/128x128.png" "$OUT/32x32.png"
    echo "  Created icon.icns"
  elif command -v sips &>/dev/null; then
    # macOS built-in
    mkdir -p /tmp/torot.iconset
    for SIZE in 16 32 64 128 256 512; do
      sips -z "$SIZE" "$SIZE" "$SOURCE" --out "/tmp/torot.iconset/icon_${SIZE}x${SIZE}.png" &>/dev/null
      sips -z $((SIZE*2)) $((SIZE*2)) "$SOURCE" --out "/tmp/torot.iconset/icon_${SIZE}x${SIZE}@2x.png" &>/dev/null
    done
    iconutil -c icns /tmp/torot.iconset -o "$OUT/icon.icns"
    rm -rf /tmp/torot.iconset
    echo "  Created icon.icns via iconutil"
  else
    # Fallback: copy PNG as placeholder
    cp "$OUT/256x256.png" "$OUT/icon.icns"
    echo "  icon.icns: copied PNG as placeholder (install png2icns for proper .icns)"
  fi

 # .ico for Windows (multi-size)
  convert "$SOURCE" \
    \( -clone 0 -resize 16x16  \) \
    \( -clone 0 -resize 24x24  \) \
    \( -clone 0 -resize 32x32  \) \
    \( -clone 0 -resize 48x48  \) \
    \( -clone 0 -resize 64x64  \) \
    \( -clone 0 -resize 128x128 \) \
    \( -clone 0 -resize 256x256 \) \
    -delete 0 "$OUT/icon.ico"
  echo "  Created icon.ico"

elif command -v vips &>/dev/null; then
  echo "Using libvips..."
  for SIZE in 32 128 256 512; do
    vips thumbnail "$SOURCE" "$OUT/${SIZE}x${SIZE}.png" "$SIZE" &>/dev/null
    echo "  Created ${SIZE}x${SIZE}.png"
  done
  cp "$OUT/256x256.png" "$OUT/128x128@2x.png"
  cp "$OUT/256x256.png" "$OUT/icon.icns"
  cp "$OUT/256x256.png" "$OUT/icon.ico"

else
  echo "No image tool found. Install ImageMagick:"
  echo "  macOS: brew install imagemagick"
  echo "  Linux: sudo apt install imagemagick"
  echo ""
  echo "Creating placeholder icons from source PNG..."
  for SIZE in 32 128 256 512; do
    cp "$SOURCE" "$OUT/${SIZE}x${SIZE}.png"
  done
  cp "$SOURCE" "$OUT/128x128@2x.png"
  cp "$SOURCE" "$OUT/icon.icns"
  cp "$SOURCE" "$OUT/icon.ico"
  echo "Placeholder icons copied. Run this script again with ImageMagick installed for proper sizes."
fi

echo ""
echo "Icons written to $OUT/"
ls -la "$OUT/"
