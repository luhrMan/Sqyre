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

# Pin flatpak-builder-tools (not master) so Flathub/CI stay reproducible.
# Bump commit + SHA256 together when upgrading the generator.
GENERATOR_COMMIT="${SQYRE_CARGO_GENERATOR_COMMIT:-f03a673abe6ce189cea1c2857e2b44af2dd79d1f}"
GENERATOR_SHA256="${SQYRE_CARGO_GENERATOR_SHA256:-b373c8ab1a05378ec5d8ed0645c7b127bcec7d2f7a1798694fbc627d570d856c}"
GENERATOR_URL="https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/${GENERATOR_COMMIT}/cargo/flatpak-cargo-generator.py"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/sqyre-flatpak"
GENERATOR="$CACHE_DIR/flatpak-cargo-generator-${GENERATOR_COMMIT}.py"
OUT="$SCRIPT_DIR/cargo-sources.json"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

sha256_file() {
  if have_cmd sha256sum; then
    sha256sum "$1" | awk '{print $1}'
  elif have_cmd shasum; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "Need sha256sum or shasum to verify flatpak-cargo-generator.py" >&2
    exit 1
  fi
}

mkdir -p "$CACHE_DIR"
need_fetch=0
if [ ! -f "$GENERATOR" ] || [ "${SQYRE_REFRESH_CARGO_GENERATOR:-}" = "1" ]; then
  need_fetch=1
elif [ "$(sha256_file "$GENERATOR")" != "$GENERATOR_SHA256" ]; then
  echo "Cached generator SHA256 mismatch; re-fetching…" >&2
  need_fetch=1
fi
if [ "$need_fetch" = "1" ]; then
  echo "Fetching flatpak-cargo-generator.py @ ${GENERATOR_COMMIT}…"
  curl -fsSL -o "$GENERATOR.partial" "$GENERATOR_URL"
  got="$(sha256_file "$GENERATOR.partial")"
  if [ "$got" != "$GENERATOR_SHA256" ]; then
    rm -f "$GENERATOR.partial"
    echo "flatpak-cargo-generator.py SHA256 mismatch (got $got, want $GENERATOR_SHA256)" >&2
    echo "Update GENERATOR_COMMIT / GENERATOR_SHA256 when bumping the pin." >&2
    exit 1
  fi
  mv -f "$GENERATOR.partial" "$GENERATOR"
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
