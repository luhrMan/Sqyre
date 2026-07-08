#!/usr/bin/env bash
# Run the full test suite without requiring an X11 display.
#
# Uses -tags=nohook so gohook is not linked (avoids segfaults when DISPLAY is unset).
# For hook/display tests (Esc via gohook, screenshot golden files), use ./scripts/test-ui.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export GOFLAGS="-tags=gocv_specific_modules,nohook -buildvcs=false"
export SQYRE_UI_TEST=1
export SQYRE_NO_HOOK=1

# Global-hook Esc tests need xvfb; run via ./scripts/test-ui.sh.
go test -skip 'TestGUIEscape' "$@" ./...
