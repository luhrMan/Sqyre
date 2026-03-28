#!/usr/bin/env bash
set -euo pipefail

# Run UI tests under a virtual X display so robotgo can initialize in headless environments.
run_tests() {
  export SQUIRE_UI_TEST=1
  go test ./ui ./ui/custom_widgets "$@"
}

if [[ -n "${DISPLAY:-}" ]]; then
  run_tests "$@"
  exit 0
fi

if command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a --server-args="-screen 0 1920x1080x24" bash -lc '
    set -euo pipefail
    export SQUIRE_UI_TEST=1
    go test ./ui ./ui/custom_widgets "$@"
  ' -- "$@"
  exit 0
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

echo "No DISPLAY is set and neither xvfb-run nor Xvfb is installed." >&2
echo "Install xvfb or run with an active display to execute UI tests." >&2
exit 1
