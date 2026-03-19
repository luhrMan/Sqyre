#!/usr/bin/env bash
# Build Sqyre AppImage; uses sqyre.AppDir and appimage-build under this directory.
# Run from anywhere: .devcontainer/builds/linux/packaging/appimage/build-appimage.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REPO_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
APP_VERSION="$(sed -n 's/^Version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$REPO_ROOT/FyneApp.toml")"
if [ -z "$APP_VERSION" ]; then
  echo "Could not read Version from $REPO_ROOT/FyneApp.toml" >&2
  exit 1
fi

RECIPE_TMP="$(mktemp)"
trap 'rm -f "$RECIPE_TMP"' EXIT
sed "s#__APP_VERSION__#$APP_VERSION#g" "$SCRIPT_DIR/AppImageBuilder.yml" > "$RECIPE_TMP"

rm -rf "$SCRIPT_DIR/sqyre.AppDir" \
       "$SCRIPT_DIR/appimage-build"

appimage-builder \
  --recipe "$RECIPE_TMP" \
  --appdir "$SCRIPT_DIR/sqyre.AppDir" \
  --build-dir "$SCRIPT_DIR/appimage-build"

echo "AppDir: $SCRIPT_DIR/sqyre.AppDir"
echo "AppImage: $SCRIPT_DIR/Sqyre-${APP_VERSION}-x86_64.AppImage"
