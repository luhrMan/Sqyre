#!/usr/bin/env bash
# Build Sqyre AppImage with gocv Mat profiling (matprofile tag).
# Same as build-appimage.sh but adds matprofile (pprof server + Mat leak logging).
# Run from repository root. Output: bin/Sqyre-<version>-matprofile-x86_64.AppImage
# See README "GoCV Mat profiling" for usage (logs, pprof URL).
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export EXTRA_GO_TAGS=matprofile
exec "$SCRIPT_DIR/build-appimage.sh"
