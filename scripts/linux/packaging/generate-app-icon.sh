#!/usr/bin/env bash
# Rasterize crates/sqyre-app/assets/icons/sqyre.svg → sqyre.png for packaging.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../lib/repo-root.sh"

SVG="$REPO_ROOT/crates/sqyre-app/assets/icons/sqyre.svg"
PNG="$REPO_ROOT/crates/sqyre-app/assets/icons/sqyre.png"
SIZE="${1:-256}"

if [ ! -f "$SVG" ]; then
  echo "ERROR: missing $SVG" >&2
  exit 1
fi

mkdir -p "$(dirname "$PNG")"

if command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w "$SIZE" -h "$SIZE" "$SVG" -o "$PNG"
elif command -v magick >/dev/null 2>&1; then
  magick -background none "$SVG" -resize "${SIZE}x${SIZE}" "$PNG"
elif command -v convert >/dev/null 2>&1; then
  convert -background none "$SVG" -resize "${SIZE}x${SIZE}" "$PNG"
else
  echo "ERROR: need rsvg-convert (librsvg2-tools) or ImageMagick convert to build sqyre.png" >&2
  exit 1
fi

echo "Wrote $PNG (${SIZE}x${SIZE})"
