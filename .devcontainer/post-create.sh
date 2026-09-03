#!/usr/bin/env bash
# Lightweight devcontainer bootstrap. Avoid cargo fetch / recursive chown of
# the whole workspace — both can OOM or stall Cursor server install.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Ownership + git safe.directory (also runs on every attach via post-start).
bash "${SCRIPT_DIR}/post-start.sh"

rustup component add rust-src 2>/dev/null || true
rustup target add wasm32-unknown-unknown 2>/dev/null || true

if docker version >/dev/null 2>&1; then
  echo "docker: ok"
else
  echo "docker: unavailable (need host Docker socket + rebuild)"
fi

echo "toolchain: $(rustc --version) / $(cargo --version)"
