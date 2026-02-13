#!/usr/bin/env bash
# Build Sqyre AppImage with AppDir and build artifacts under this directory.
# Run from anywhere: .devcontainer/builds/linux/packaging/build-appimage.sh

set -e
PACKAGING_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PACKAGING_DIR"

appimage-builder \
  --recipe AppImageBuilder.yml \
  --appdir "$PACKAGING_DIR/AppDir" \
  --build-dir "$PACKAGING_DIR/appimage-build"

echo "AppDir: $PACKAGING_DIR/AppDir"
echo "AppImage: $PACKAGING_DIR/Sqyre-0.5.0-x86_64.AppImage"
