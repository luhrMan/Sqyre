#!/bin/bash
# Cross-compile Sqyre for Windows. Run from repository root.
# Builds the Docker image (OpenCV4 + MinGW sysroot) then runs fyne-cross.
set -e
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"
echo "Building Windows cross-compile image (OpenCV4 + MinGW)..."
docker build -f .devcontainer/builds/windows/Dockerfile.windows-amd64 -t fyne-cross-windows:local .
echo "Cross-compiling Sqyre for Windows..."
fyne-cross windows -image fyne-cross-windows:local --app-id com.sqyre.app ./cmd/sqyre