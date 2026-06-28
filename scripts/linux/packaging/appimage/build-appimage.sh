#!/usr/bin/env bash
# Build Sqyre AppImage; uses sqyre.AppDir and appimage-build under this directory.
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

# Recipe must live under this directory: appimage-builder sets SOURCE_DIR to the
# recipe file's parent. A /tmp path makes SOURCE_DIR=/tmp and breaks `go build`.
RECIPE_TMP="$(mktemp -p "$SCRIPT_DIR" .AppImageBuilder.XXXXXX.yml)"
TOOLS_DIR="$(mktemp -d)"
trap 'rm -f "$RECIPE_TMP"; rm -rf "$TOOLS_DIR"' EXIT
BUILD_TAGS="${BUILD_TAGS:-gocv_specific_modules}"
if [ -n "${EXTRA_GO_TAGS:-}" ]; then
  BUILD_TAGS="$BUILD_TAGS,$EXTRA_GO_TAGS"
fi
APP_SUFFIX=""
if [[ ",$BUILD_TAGS," == *",matprofile,"* ]]; then
  APP_SUFFIX="-matprofile"
fi
sed -e "s#__APP_VERSION__#$APP_VERSION#g" \
    -e "s#__BUILD_TAGS__#$BUILD_TAGS#g" \
    -e "s#__APP_SUFFIX__#$APP_SUFFIX#g" \
    "$SCRIPT_DIR/AppImageBuilder.yml" > "$RECIPE_TMP"

# appimage-builder calls mksquashfs with xattrs enabled; POSIX ACLs on the AppDir
# (common on bind-mounted repos with default ACLs) become system.posix_acl_* xattrs
# and trigger noisy "Unrecognised xattr prefix" lines. AppImages do not need those.
REAL_MK="$(command -v mksquashfs)"
printf '#!/usr/bin/env bash\nexec %q "$@" -no-xattrs\n' "$REAL_MK" >"$TOOLS_DIR/mksquashfs"
chmod +x "$TOOLS_DIR/mksquashfs"
export PATH="$TOOLS_DIR:$PATH"

rm -rf "$SCRIPT_DIR/sqyre.AppDir" \
       "$SCRIPT_DIR/appimage-build"

appimage-builder \
  --recipe "$RECIPE_TMP" \
  --appdir "$SCRIPT_DIR/sqyre.AppDir" \
  --build-dir "$SCRIPT_DIR/appimage-build"

OUT_DIR="$REPO_ROOT/bin"
mkdir -p "$OUT_DIR"
APP_IMAGE_NAME="Sqyre-${APP_VERSION}${APP_SUFFIX}-x86_64.AppImage"
mv -f "$SCRIPT_DIR/$APP_IMAGE_NAME" "$OUT_DIR/$APP_IMAGE_NAME"

echo "AppDir: $SCRIPT_DIR/sqyre.AppDir"
echo "AppImage: $OUT_DIR/$APP_IMAGE_NAME"
