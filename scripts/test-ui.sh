#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export GOFLAGS="${GOFLAGS:--tags=gocv_specific_modules -buildvcs=false}"

TEST_TIMEOUT="${SQYRE_TEST_TIMEOUT:-5m}"

display_usable() {
  [[ -n "${DISPLAY:-}" ]] && command -v xdpyinfo >/dev/null 2>&1 && xdpyinfo >/dev/null 2>&1
}

ensure_x11_socket() {
  mkdir -p /tmp/.X11-unix
  chmod 1777 /tmp/.X11-unix 2>/dev/null || true
}

has_run_filter() {
  local arg
  for arg in "$@"; do
    case "$arg" in
      -run|-run=*|-skip|-skip=*)
        return 0
        ;;
    esac
  done
  return 1
}

run_tests() {
  export SQYRE_UI_TEST=1
  unset SQYRE_NO_HOOK

  echo "test-ui: running ./ui/ (timeout ${TEST_TIMEOUT})..."
  go test -timeout "${TEST_TIMEOUT}" -v "$@" ./ui/

  if has_run_filter "$@"; then
    echo "test-ui: skipping auxiliary packages (narrow -run/-skip filter)"
    return
  fi

  echo "test-ui: running ./ui/custom_widgets/..."
  go test -timeout "${TEST_TIMEOUT}" -v "$@" ./ui/custom_widgets/

  echo "test-ui: running ./internal/models/actions/..."
  go test -timeout "${TEST_TIMEOUT}" -v "$@" ./internal/models/actions/

  echo "test-ui: running ./internal/services/ (TestExecute*)..."
  go test -timeout "${TEST_TIMEOUT}" -v ./internal/services/ -run 'TestExecute_'
}

if [[ "${1:-}" == "--xvfb-inner" ]]; then
  shift
  ensure_x11_socket
  run_tests "$@"
  exit 0
fi

if display_usable; then
  ensure_x11_socket
  run_tests "$@"
  exit 0
fi

ensure_x11_socket

if command -v Xvfb >/dev/null 2>&1; then
  echo "test-ui: starting Xvfb on :99..."
  Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp >/tmp/sqyre-xvfb.log 2>&1 &
  XVFB_PID=$!
  trap 'kill "${XVFB_PID}" >/dev/null 2>&1 || true' EXIT
  sleep 0.5
  export DISPLAY=:99
  run_tests "$@"
  exit 0
fi

if command -v xvfb-run >/dev/null 2>&1; then
  echo "test-ui: launching xvfb-run..."
  exec xvfb-run -a --server-args="-screen 0 1920x1080x24" "$0" --xvfb-inner "$@"
fi

echo "No usable DISPLAY and neither xvfb-run nor Xvfb is installed." >&2
echo "Install xvfb or run with an active display to execute UI tests." >&2
exit 1
