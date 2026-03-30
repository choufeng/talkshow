#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ICONS_SRC="$PROJECT_DIR/src-tauri/icons-src"
ICONS_DIR="$PROJECT_DIR/src-tauri/icons"
ICONSET_DIR="$PROJECT_DIR/src-tauri/icons/icon.iconset"

mkdir -p "$ICONS_DIR"

echo "=== Converting SVGs to PNG with rsvg-convert ==="

MAIN_SVG="$ICONS_SRC/icon.svg"
TRAY_SVG="$ICONS_SRC/tray-idle.svg"
MAIN_PNG="/tmp/talkshow-icon-base.png"
TRAY_PNG="/tmp/talkshow-tray-base.png"

rsvg-convert -w 1024 -h 1024 "$MAIN_SVG" -o "$MAIN_PNG"
rsvg-convert -w 64 -h 64 "$TRAY_SVG" -o "$TRAY_PNG"
echo "Base PNGs generated."

echo ""
echo "=== Generating app icons ==="

generate_png() {
  local size="$1"
  local output="$ICONS_DIR/$2"
  magick "$MAIN_PNG" -resize "${size}x${size}" -quality 100 "$output"
  echo "Generated: $output (${size}x${size})"
}

generate_png 1024 "icon.png"
generate_png 32 "32x32.png"
generate_png 128 "128x128.png"
generate_png 256 "128x128@2x.png"
generate_png 50 "StoreLogo.png"
generate_png 30 "Square30x30Logo.png"
generate_png 44 "Square44x44Logo.png"
generate_png 71 "Square71x71Logo.png"
generate_png 89 "Square89x89Logo.png"
generate_png 107 "Square107x107Logo.png"
generate_png 142 "Square142x142Logo.png"
generate_png 150 "Square150x150Logo.png"
generate_png 284 "Square284x284Logo.png"
generate_png 310 "Square310x310Logo.png"

echo ""
echo "=== Generating macOS iconset ==="

mkdir -p "$ICONSET_DIR"
magick "$MAIN_PNG" -resize 16x16 "$ICONSET_DIR/icon_16x16.png"
magick "$MAIN_PNG" -resize 32x32 "$ICONSET_DIR/icon_16x16@2x.png"
magick "$MAIN_PNG" -resize 32x32 "$ICONSET_DIR/icon_32x32.png"
magick "$MAIN_PNG" -resize 64x64 "$ICONSET_DIR/icon_32x32@2x.png"
magick "$MAIN_PNG" -resize 128x128 "$ICONSET_DIR/icon_128x128.png"
magick "$MAIN_PNG" -resize 256x256 "$ICONSET_DIR/icon_128x128@2x.png"
magick "$MAIN_PNG" -resize 256x256 "$ICONSET_DIR/icon_256x256.png"
magick "$MAIN_PNG" -resize 512x512 "$ICONSET_DIR/icon_256x256@2x.png"
magick "$MAIN_PNG" -resize 512x512 "$ICONSET_DIR/icon_512x512.png"
magick "$MAIN_PNG" -resize 1024x1024 "$ICONSET_DIR/icon_512x512@2x.png"

rm -f "$ICONS_DIR/icon.icns"
iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/icon.icns"
rm -rf "$ICONSET_DIR"
echo "Generated: icon.icns"

echo ""
echo "=== Generating Windows ICO ==="

magick "$MAIN_PNG" \
  \( -clone 0 -resize 16x16 \) \
  \( -clone 0 -resize 32x32 \) \
  \( -clone 0 -resize 48x48 \) \
  \( -clone 0 -resize 64x64 \) \
  \( -clone 0 -resize 128x128 \) \
  \( -clone 0 -resize 256x256 \) \
  -delete 0 \
  "$ICONS_DIR/icon.ico"
echo "Generated: icon.ico"

echo ""
echo "=== Generating tray icons ==="

magick "$TRAY_PNG" -resize 32x32 "$ICONS_DIR/icon-tray-idle-1x.png"
magick "$TRAY_PNG" -resize 64x64 "$ICONS_DIR/icon-tray-idle-2x.png"
echo "Generated: tray idle icons (1x + 2x)"

magick "$TRAY_PNG" -resize 32x32 \
  \( -clone 0 -fill "#FF3B30" -draw "circle 26,6 26,3" \) \
  -flatten "$ICONS_DIR/recording.png"
magick "$TRAY_PNG" -resize 64x64 \
  \( -clone 0 -fill "#FF3B30" -draw "circle 52,12 52,6" \) \
  -flatten "$ICONS_DIR/recording@2x.png"
echo "Generated: recording tray icons (1x + 2x)"

rm -f "$MAIN_PNG" "$TRAY_PNG"

echo ""
echo "=== Done! All icons generated. ==="
ls -la "$ICONS_DIR"/*.png "$ICONS_DIR"/*.icns "$ICONS_DIR"/*.ico
