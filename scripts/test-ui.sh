#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export GOFLAGS="${GOFLAGS:--tags=gocv_specific_modules -buildvcs=false}"

display_usable() {
  [[ -n "${DISPLAY:-}" ]] && command -v xdpyinfo >/dev/null 2>&1 && xdpyinfo >/dev/null 2>&1
}

run_tests() {
  export SQUIRE_UI_TEST=1
  unset SQYRE_NO_HOOK
  go test -v "$@" ./ui/
  go test -v "$@" ./ui/custom_widgets/
  go test -v "$@" ./internal/models/actions/
  go test -v "$@" ./internal/services/ -run '^TestExecute'
}

if [[ "${1:-}" == "--xvfb-inner" ]]; then
  shift
  run_tests "$@"
  exit 0
fi

if display_usable; then
  run_tests "$@"
  exit 0
fi

if command -v xvfb-run >/dev/null 2>&1; then
  exec xvfb-run -a --server-args="-screen 0 1920x1080x24" "$0" --xvfb-inner "$@"
fi

if command -v Xvfb >/dev/null 2>&1; then
  Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp >/tmp/sqyre-xvfb.log 2>&1 &
  XVFB_PID=$!
  trap 'kill "${XVFB_PID}" >/dev/null 2>&1 || true' EXIT
  sleep 0.5
  export DISPLAY=:99
  run_tests "$@"
  exit 0
fi

echo "No usable DISPLAY and neither xvfb-run nor Xvfb is installed." >&2
echo "Install xvfb or run with an active display to execute UI tests." >&2
exit 1
