#!/usr/bin/env bash
# Run UI and unit tests under a virtual X display (robotgo / Fyne headless).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export SQUIRE_UI_TEST=1
export SQYRE_NO_HOOK=1
export GOFLAGS="${GOFLAGS:--tags=gocv_specific_modules -buildvcs=false}"

if ! command -v xvfb-run >/dev/null 2>&1; then
  echo "xvfb-run not found; install xvfb (e.g. apt install xvfb)" >&2
  exit 1
fi

exec xvfb-run -a go test -v "$@" \
  ./ui/ \
  ./internal/models/actions/

xvfb-run -a go test -v "$@" ./internal/services/ -run '^TestExecute'
