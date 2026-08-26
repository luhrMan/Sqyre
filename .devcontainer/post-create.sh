#!/usr/bin/env bash
# Lightweight devcontainer bootstrap. Avoid cargo fetch / recursive chown here —
# both can OOM or stall Cursor server install on large caches.
set -euo pipefail

if [ ! -w "${CARGO_HOME:-/home/vscode/.cargo}" ]; then
  sudo chown -R vscode:vscode "${CARGO_HOME:-/home/vscode/.cargo}" 2>/dev/null || true
fi

rustup component add rust-src 2>/dev/null || true
rustup target add wasm32-unknown-unknown 2>/dev/null || true

if docker version >/dev/null 2>&1; then
  echo "docker: ok"
else
  echo "docker: unavailable (need host Docker socket + rebuild)"
fi

echo "toolchain: $(rustc --version) / $(cargo --version)"
