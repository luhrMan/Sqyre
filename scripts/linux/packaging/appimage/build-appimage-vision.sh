#!/usr/bin/env bash
# Build Sqyre AppImage with sqyre-vision sidecar (embedded ONNX models).
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../../lib/repo-root.sh"
APP_VERSION="$(sed -n 's/^Version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$REPO_ROOT/FyneApp.toml")"
if [ -z "$APP_VERSION" ]; then
  echo "Could not read Version from $REPO_ROOT/FyneApp.toml" >&2
  exit 1
fi

RECIPE_TMP="$(mktemp -p "$SCRIPT_DIR" .AppImageBuilder-vision.XXXXXX.yml)"
TOOLS_DIR="$(mktemp -d)"
trap 'rm -f "$RECIPE_TMP"; rm -rf "$TOOLS_DIR"' EXIT
BUILD_TAGS="${BUILD_TAGS:-gocv_specific_modules}"
if [ -n "${EXTRA_GO_TAGS:-}" ]; then
  BUILD_TAGS="$BUILD_TAGS,$EXTRA_GO_TAGS"
fi
APP_SUFFIX="-vision"
sed -e "s#__APP_VERSION__#$APP_VERSION#g" \
    -e "s#__BUILD_TAGS__#$BUILD_TAGS#g" \
    -e "s#__APP_SUFFIX__#$APP_SUFFIX#g" \
    "$SCRIPT_DIR/AppImageBuilder-vision.yml" > "$RECIPE_TMP"

REAL_MK="$(command -v mksquashfs)"
printf '#!/usr/bin/env bash\nexec %q "$@" -no-xattrs\n' "$REAL_MK" >"$TOOLS_DIR/mksquashfs"
chmod +x "$TOOLS_DIR/mksquashfs"
export PATH="$TOOLS_DIR:$PATH"

rm -rf "$SCRIPT_DIR/sqyre.AppDir" "$SCRIPT_DIR/appimage-build"

appimage-builder \
  --recipe "$RECIPE_TMP" \
  --appdir "$SCRIPT_DIR/sqyre.AppDir" \
  --build-dir "$SCRIPT_DIR/appimage-build"

OUT_DIR="$REPO_ROOT/bin"
mkdir -p "$OUT_DIR"
APP_IMAGE_NAME="Sqyre-Vision-${APP_VERSION}${APP_SUFFIX}-x86_64.AppImage"
mv -f "$SCRIPT_DIR/$APP_IMAGE_NAME" "$OUT_DIR/$APP_IMAGE_NAME" 2>/dev/null || \
  mv -f "$SCRIPT_DIR/Sqyre-Vision-${APP_VERSION}-vision-x86_64.AppImage" "$OUT_DIR/Sqyre-Vision-${APP_VERSION}-vision-x86_64.AppImage"

echo "AppImage: $OUT_DIR/Sqyre-Vision-${APP_VERSION}-vision-x86_64.AppImage"
