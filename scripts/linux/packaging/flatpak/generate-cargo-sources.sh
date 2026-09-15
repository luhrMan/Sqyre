#!/usr/bin/env bash
# Regenerate cargo-sources.json from the workspace Cargo.lock (Flathub offline builds).
#
# Requires network + Python deps: aiohttp, tomlkit
#   pip3 install --user 'aiohttp>=3.9.5,<4' 'tomlkit>=0.13.3,<1'
#
# Run from repo root or this directory:
#   scripts/linux/packaging/flatpak/generate-cargo-sources.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../../lib/repo-root.sh"

GENERATOR_URL="https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/sqyre-flatpak"
GENERATOR="$CACHE_DIR/flatpak-cargo-generator.py"
OUT="$SCRIPT_DIR/cargo-sources.json"

mkdir -p "$CACHE_DIR"
if [ ! -f "$GENERATOR" ] || [ "${SQYRE_REFRESH_CARGO_GENERATOR:-}" = "1" ]; then
  echo "Fetching flatpak-cargo-generator.py…"
  curl -fsSL -o "$GENERATOR" "$GENERATOR_URL"
fi

need_py() {
  python3 -c "import aiohttp, tomlkit" 2>/dev/null || {
    echo "ERROR: need Python packages aiohttp and tomlkit." >&2
    echo "  pip3 install --user 'aiohttp>=3.9.5,<4' 'tomlkit>=0.13.3,<1'" >&2
    exit 1
  }
}
need_py

echo "Generating $OUT from Cargo.lock…"
python3 "$GENERATOR" "$REPO_ROOT/Cargo.lock" -o "$OUT"
echo "Wrote $(wc -c < "$OUT") bytes"
