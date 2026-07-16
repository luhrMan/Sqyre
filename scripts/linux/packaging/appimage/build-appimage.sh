#!/usr/bin/env bash
# Build Sqyre AppImage (Rust); uses sqyre.AppDir and appimage-build under this directory.
#
# Prefer a native build when appimage-builder + squashfs-tools are installed
# (devcontainer). Otherwise re-run inside the project Docker image when Docker
# is available (same path CI uses).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../../lib/repo-root.sh"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

# Version: RELEASE_VERSION env (CI), else VERSION file, else Cargo.toml.
APP_VERSION="${RELEASE_VERSION:-}"
if [ -z "$APP_VERSION" ] && [ -f "$REPO_ROOT/VERSION" ]; then
  APP_VERSION="$(tr -d '[:space:]' < "$REPO_ROOT/VERSION")"
fi
if [ -z "$APP_VERSION" ]; then
  APP_VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$REPO_ROOT/rust/crates/sqyre-app/Cargo.toml" | head -1)"
fi
if [ -z "$APP_VERSION" ]; then
  echo "Could not determine app version (set RELEASE_VERSION or write VERSION)" >&2
  exit 1
fi
export RELEASE_VERSION="$APP_VERSION"

need_native_tools() {
  have_cmd appimage-builder && have_cmd mksquashfs && have_cmd patchelf && have_cmd cargo
}

run_native() {
  # Recipe must live under this directory: appimage-builder sets SOURCE_DIR to the
  # recipe file's parent.
  RECIPE_TMP="$(mktemp -p "$SCRIPT_DIR" .AppImageBuilder.XXXXXX.yml)"
  TOOLS_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sqyre-appimage-tools.XXXXXX")"
  cleanup() {
    rm -f "$RECIPE_TMP"
    rm -rf "$TOOLS_DIR"
  }
  trap cleanup EXIT

  sed -e "s#__APP_VERSION__#$APP_VERSION#g" \
      "$SCRIPT_DIR/AppImageBuilder.yml" > "$RECIPE_TMP"

  # appimage-builder calls mksquashfs with xattrs enabled; POSIX ACLs on the AppDir
  # become system.posix_acl_* xattrs and trigger noisy "Unrecognised xattr prefix" lines.
  REAL_MK="$(command -v mksquashfs)"
  printf '#!/usr/bin/env bash\nexec %q "$@" -no-xattrs\n' "$REAL_MK" >"$TOOLS_DIR/mksquashfs"
  chmod +x "$TOOLS_DIR/mksquashfs"
  export PATH="$TOOLS_DIR:$PATH"

  # Prefer workspace-local rustup/cargo when present (host + CI cache layouts).
  if [ -z "${CARGO_HOME:-}" ] && [ -x "$REPO_ROOT/.cargo-home/bin/cargo" ]; then
    export CARGO_HOME="$REPO_ROOT/.cargo-home"
    export PATH="$CARGO_HOME/bin:$PATH"
  fi
  if [ -z "${RUSTUP_HOME:-}" ] && [ -d "$REPO_ROOT/.rustup-home" ]; then
    export RUSTUP_HOME="$REPO_ROOT/.rustup-home"
  fi
  if [ -z "${CARGO_TARGET_DIR:-}" ]; then
    export CARGO_TARGET_DIR="$REPO_ROOT/rust/target"
  fi

  rm -rf "$SCRIPT_DIR/sqyre.AppDir" \
         "$SCRIPT_DIR/appimage-build"

  echo "Building AppImage v${APP_VERSION} (native)…"
  appimage-builder \
    --recipe "$RECIPE_TMP" \
    --appdir "$SCRIPT_DIR/sqyre.AppDir" \
    --build-dir "$SCRIPT_DIR/appimage-build"

  OUT_DIR="$REPO_ROOT/bin"
  mkdir -p "$OUT_DIR"
  APP_IMAGE_NAME="Sqyre-${APP_VERSION}-x86_64.AppImage"
  if [ ! -f "$SCRIPT_DIR/$APP_IMAGE_NAME" ]; then
    echo "Expected AppImage not found: $SCRIPT_DIR/$APP_IMAGE_NAME" >&2
    ls -la "$SCRIPT_DIR"/*.AppImage 2>/dev/null || true
    exit 1
  fi
  mv -f "$SCRIPT_DIR/$APP_IMAGE_NAME" "$OUT_DIR/$APP_IMAGE_NAME"

  echo "AppDir: $SCRIPT_DIR/sqyre.AppDir"
  echo "AppImage: $OUT_DIR/$APP_IMAGE_NAME"
}

run_docker() {
  if ! have_cmd docker; then
    echo "AppImage tools missing (need appimage-builder, mksquashfs, patchelf, cargo)." >&2
    echo "Install them, use the devcontainer, or install Docker for the fallback build." >&2
    exit 1
  fi

  IMAGE="${SQYRE_APPIMAGE_IMAGE:-sqyre-linux-build:latest}"
  DOCKERFILE="$REPO_ROOT/.devcontainer/Dockerfile"

  if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    echo "Building Docker image $IMAGE (one-time)…"
    docker build -f "$DOCKERFILE" -t "$IMAGE" "$REPO_ROOT"
  fi

  # Ensure tessdata exists so the recipe can bundle it.
  if [ ! -f "$REPO_ROOT/assets/tessdata/eng.traineddata" ]; then
    echo "Downloading eng.traineddata…"
    "$REPO_ROOT/scripts/download-tessdata.sh"
  fi

  mkdir -p "$REPO_ROOT/.cache/cargo" "$REPO_ROOT/rust/target" "$REPO_ROOT/bin"

  echo "Building AppImage v${APP_VERSION} (docker: $IMAGE)…"
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -v "$REPO_ROOT:/workspace" -w /workspace \
    -e HOME=/tmp \
    -e CARGO_HOME=/workspace/.cache/cargo \
    -e CARGO_TARGET_DIR=/workspace/rust/target \
    -e RUSTUP_HOME=/usr/local/rustup \
    -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
    -e RELEASE_VERSION="$APP_VERSION" \
    -e SQYRE_APPIMAGE_FORCE_NATIVE=1 \
    "$IMAGE" \
    bash -c 'set -euo pipefail; scripts/linux/packaging/appimage/build-appimage.sh'

  OUT="$REPO_ROOT/bin/Sqyre-${APP_VERSION}-x86_64.AppImage"
  if [ ! -f "$OUT" ]; then
    echo "Docker AppImage build finished but $OUT is missing" >&2
    exit 1
  fi
  echo "AppImage: $OUT"
}

# Avoid recursive docker when already inside the build image.
if [ "${SQYRE_APPIMAGE_FORCE_NATIVE:-}" = "1" ] || need_native_tools; then
  if ! need_native_tools; then
    echo "SQYRE_APPIMAGE_FORCE_NATIVE=1 but required tools are missing." >&2
    echo "Need: appimage-builder, mksquashfs, patchelf, cargo" >&2
    exit 1
  fi
  run_native
else
  echo "Native AppImage tools not found; using Docker fallback…"
  run_docker
fi
