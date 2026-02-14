#!/usr/bin/env bash
# Build Sqyre AppImage; uses sqyre.AppDir and appimage-build under this directory.
# Run from anywhere: .devcontainer/builds/linux/packaging/appimage/build-appimage.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

rm -rf "$SCRIPT_DIR/sqyre.AppDir/usr/lib" \
       "$SCRIPT_DIR/sqyre.AppDir/usr/bin" \
       "$SCRIPT_DIR/sqyre.AppDir/lib" \
       "$SCRIPT_DIR/appimage-build"

appimage-builder \
  --recipe AppImageBuilder.yml \
  --appdir "$SCRIPT_DIR/sqyre.AppDir" \
  --build-dir "$SCRIPT_DIR/appimage-build"

echo "AppDir: $SCRIPT_DIR/sqyre.AppDir"
echo "AppImage: $SCRIPT_DIR/Sqyre-0.5.0-x86_64.AppImage"
