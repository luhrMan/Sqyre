#!/usr/bin/env bash
# Generate internal/assets/icons/sqyre-256.png from the SVG for Flatpak/AppStream.
# AppStream compose can fail with file-read-error on SVG in some environments (e.g. Nix);
# a 256x256 PNG avoids that. Run from repo root and commit the generated file.
set -e
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../.." && pwd)"
SVG="$REPO_ROOT/internal/assets/icons/sqyre.svg"
OUT="$REPO_ROOT/internal/assets/icons/sqyre-256.png"
if [ ! -f "$SVG" ]; then
  echo "Error: $SVG not found" >&2
  exit 1
fi
if command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w 256 -h 256 "$SVG" -o "$OUT"
elif command -v convert >/dev/null 2>&1; then
  convert "$SVG" -resize 256x256 "$OUT"
else
  echo "Error: need rsvg-convert (librsvg) or convert (ImageMagick). Install one and re-run." >&2
  exit 1
fi
echo "Generated $OUT"
