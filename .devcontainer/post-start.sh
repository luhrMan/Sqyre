#!/usr/bin/env bash
# Cheap per-attach fixes for bind-mount UID / ownership drift across hosts.
# Avoid recursive chown of the whole tree — only fix known writable caches.
set -euo pipefail

# Git refuses worktrees when the bind-mount owner UID ≠ the container user
# (common when updateRemoteUserUID remaps late, or host UID ≠ image UID).
if command -v git >/dev/null 2>&1; then
  if ! git config --global --get-all safe.directory 2>/dev/null | grep -qx '\*'; then
    git config --global --add safe.directory '*'
  fi
fi

fix_writable() {
  local p="$1"
  [ -e "$p" ] || return 0
  [ -w "$p" ] && return 0
  sudo chown -R "$(id -u):$(id -g)" "$p" 2>/dev/null || true
}

fix_writable "${CARGO_HOME:-/home/vscode/.cargo}"
fix_writable /workspace/target
fix_writable /workspace/.cache
fix_writable /workspace/bin
fix_writable /workspace/.cargo-home
