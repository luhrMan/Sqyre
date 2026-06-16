#!/bin/bash
# Cross-compile Sqyre for Windows (amd64) with gocv Mat profiling enabled.
# Same as build.sh but adds -tags matprofile (pprof server + Mat leak logging).
# Run from repository root. Output: bin/windows-amd64/Sqyre.exe (or BIN_DIR if set).
# See README "GoCV Mat profiling" for usage (logs, pprof URL).
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export EXTRA_GO_TAGS=matprofile
exec "$SCRIPT_DIR/build.sh"
