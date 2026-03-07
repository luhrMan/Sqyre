#!/bin/bash
# Cross-compile Sqyre for Windows (amd64) with gocv Mat profiling enabled.
# Same as build.sh but adds -tags matprofile (pprof server + Mat leak logging).
# Run from repository root. Output: .devcontainer/builds/windows/output/Sqyre.exe
# See README "GoCV Mat profiling" for usage (logs, pprof URL).
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
export EXTRA_GO_TAGS=matprofile
exec "$SCRIPT_DIR/build.sh"
