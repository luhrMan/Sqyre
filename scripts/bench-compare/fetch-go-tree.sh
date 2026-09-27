#!/usr/bin/env bash
# Fetch the last Go/Fyne Sqyre tree (pre Rust cutover) into .cache/go-sqyre.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST="${GO_SQYRE_ROOT:-$ROOT/.cache/go-sqyre}"
# Parent of "Complete Rust cutover by removing the Go/Fyne codebase."
GO_REV="${GO_SQYRE_REV:-4b2a2cbe60e231f2c96a113013213e4874d9d503}"

mkdir -p "$(dirname "$DEST")"
if [[ -f "$DEST/go.mod" ]]; then
  cur="$(git -C "$DEST" rev-parse HEAD 2>/dev/null || true)"
  if [[ "$cur" == "$GO_REV" ]]; then
    echo "Go tree already at $GO_REV → $DEST"
    exit 0
  fi
fi

echo "Fetching Go Sqyre tree $GO_REV → $DEST"
rm -rf "$DEST"
git -C "$ROOT" worktree remove --force "$DEST" 2>/dev/null || true
git -C "$ROOT" worktree add --detach "$DEST" "$GO_REV"
echo "Ready: $DEST"
