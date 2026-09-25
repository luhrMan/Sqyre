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
# shellcheck source=scripts/lib/docker-host-path.sh
. "$SCRIPT_DIR/../../../lib/docker-host-path.sh"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

# Version: RELEASE_VERSION env (CI), else VERSION file, else Cargo.toml.
APP_VERSION="${RELEASE_VERSION:-}"
if [ -z "$APP_VERSION" ] && [ -f "$REPO_ROOT/VERSION" ]; then
  APP_VERSION="$(tr -d '[:space:]' < "$REPO_ROOT/VERSION")"
fi
if [ -z "$APP_VERSION" ]; then
  APP_VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$REPO_ROOT/crates/sqyre-app/Cargo.toml" | head -1)"
fi
if [ -z "$APP_VERSION" ]; then
  echo "Could not determine app version (set RELEASE_VERSION or write VERSION)" >&2
  exit 1
fi
export RELEASE_VERSION="$APP_VERSION"

need_native_tools() {
  have_cmd appimage-builder && have_cmd mksquashfs && have_cmd patchelf && have_cmd cargo
}

# appimage-builder hardcodes squashfs -comp xz and the old AppImageKit runtime
# (needs system libfuse.so.2). That makes cold start multi-second when FUSE is
# missing (extract-and-run) and slower even with FUSE (xz random reads).
# We --skip-appimage and repack the AppDir with type2-runtime + zstd instead.
#
# type2-runtime continuous build (commit 75849dc). Override URL/SHA via env when bumping.
TYPE2_RUNTIME_URL="${SQYRE_APPIMAGE_RUNTIME_URL:-https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64}"
TYPE2_RUNTIME_SHA256="${SQYRE_APPIMAGE_RUNTIME_SHA256:-1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf}"

sha256_file() {
  if have_cmd sha256sum; then
    sha256sum "$1" | awk '{print $1}'
  elif have_cmd shasum; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "Need sha256sum or shasum to verify type2-runtime" >&2
    exit 1
  fi
}

ensure_type2_runtime() {
  local cache_dir="$SCRIPT_DIR/.runtime-cache"
  local runtime="$cache_dir/runtime-x86_64"
  local got
  mkdir -p "$cache_dir"
  if [ -x "$runtime" ] && [ -s "$runtime" ]; then
    got="$(sha256_file "$runtime")"
    if [ "$got" = "$TYPE2_RUNTIME_SHA256" ]; then
      printf '%s\n' "$runtime"
      return 0
    fi
    echo "Cached type2-runtime SHA256 mismatch (got $got, want $TYPE2_RUNTIME_SHA256); re-downloading…" >&2
    rm -f "$runtime"
  fi
  echo "Downloading AppImage type2-runtime…" >&2
  if have_cmd curl; then
    curl -fsSL -o "$runtime.partial" "$TYPE2_RUNTIME_URL"
  elif have_cmd wget; then
    wget -q -O "$runtime.partial" "$TYPE2_RUNTIME_URL"
  else
    echo "Need curl or wget to download type2-runtime from $TYPE2_RUNTIME_URL" >&2
    exit 1
  fi
  got="$(sha256_file "$runtime.partial")"
  if [ "$got" != "$TYPE2_RUNTIME_SHA256" ]; then
    rm -f "$runtime.partial"
    echo "type2-runtime SHA256 mismatch (got $got, want $TYPE2_RUNTIME_SHA256)" >&2
    echo "Update TYPE2_RUNTIME_SHA256 / SQYRE_APPIMAGE_RUNTIME_SHA256 when bumping the runtime." >&2
    exit 1
  fi
  chmod +x "$runtime.partial"
  mv -f "$runtime.partial" "$runtime"
  printf '%s\n' "$runtime"
}

# AppRun v2 bakes the absolute build AppDir into APPDIR_PATH_MAPPINGS; strip it so
# the shipped image does not redirect through a CI/dev path if that path exists.
scrub_appdir_path_mappings() {
  local env_file="$1/AppRun.env"
  [ -f "$env_file" ] || return 0
  if grep -q '^APPDIR_PATH_MAPPINGS=' "$env_file"; then
    # Keep the key (AppRun may expect it) but clear absolute build-host mappings.
    sed -i 's|^APPDIR_PATH_MAPPINGS=.*|APPDIR_PATH_MAPPINGS=|' "$env_file"
  fi
}

# Replace the xz AppImage from appimage-builder with a fast-start payload.
repack_fast_appimage() {
  local appdir="$1"
  local out_path="$2"
  local runtime payload tmp_out
  if [ ! -x "$appdir/AppRun" ] && [ ! -x "$appdir/usr/bin/sqyre" ]; then
    echo "AppDir incomplete (missing AppRun / usr/bin/sqyre): $appdir" >&2
    exit 1
  fi
  scrub_appdir_path_mappings "$appdir"
  runtime="$(ensure_type2_runtime)"
  payload="$(mktemp "${TMPDIR:-/tmp}/sqyre-appimage-payload.XXXXXX.squashfs")"
  tmp_out="$(mktemp "${TMPDIR:-/tmp}/sqyre-appimage-out.XXXXXX")"

  echo "Repacking AppImage (zstd + type2-runtime)…"
  if ! mksquashfs "$appdir" "$payload" \
    -root-owned \
    -noappend \
    -no-xattrs \
    -comp zstd \
    -Xcompression-level 3 \
    >/dev/null
  then
    rm -f "$payload" "$tmp_out"
    echo "mksquashfs failed while repacking AppImage" >&2
    exit 1
  fi
  cat "$runtime" "$payload" >"$tmp_out"
  chmod 755 "$tmp_out"
  mv -f "$tmp_out" "$out_path"
  rm -f "$payload"
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
    export CARGO_TARGET_DIR="$REPO_ROOT/target"
  fi

  rm -rf "$SCRIPT_DIR/sqyre.AppDir" \
         "$SCRIPT_DIR/appimage-build"

  echo "Building AppImage v${APP_VERSION} (native)…"
  # Skip appimage-builder's xz + AppImageKit prime; we repack from AppDir below.
  appimage-builder \
    --recipe "$RECIPE_TMP" \
    --appdir "$SCRIPT_DIR/sqyre.AppDir" \
    --build-dir "$SCRIPT_DIR/appimage-build" \
    --skip-appimage

  OUT_DIR="$REPO_ROOT/bin"
  mkdir -p "$OUT_DIR"
  APP_IMAGE_NAME="Sqyre-${APP_VERSION}-x86_64.AppImage"
  if [ ! -d "$SCRIPT_DIR/sqyre.AppDir" ]; then
    echo "Expected AppDir not found: $SCRIPT_DIR/sqyre.AppDir" >&2
    exit 1
  fi
  repack_fast_appimage "$SCRIPT_DIR/sqyre.AppDir" "$OUT_DIR/$APP_IMAGE_NAME"

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

  # Share crates with host/CI: prefer caller CARGO_HOME when it lives in-repo
  # (Makefile sets .cargo-home), else the CI/docker registry cache (.cache/cargo).
  CARGO_HOME_REL=".cache/cargo"
  if [ -n "${CARGO_HOME:-}" ]; then
    case "$CARGO_HOME" in
      "$REPO_ROOT"/*) CARGO_HOME_REL="${CARGO_HOME#"$REPO_ROOT"/}" ;;
    esac
  elif [ -x "$REPO_ROOT/.cargo-home/bin/cargo" ]; then
    CARGO_HOME_REL=".cargo-home"
  fi
  mkdir -p "$REPO_ROOT/$CARGO_HOME_REL" "$REPO_ROOT/target" "$REPO_ROOT/bin"

  echo "Building AppImage v${APP_VERSION} (docker: $IMAGE, CARGO_HOME=$CARGO_HOME_REL)…"
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -v "$(docker_host_path "$REPO_ROOT"):/workspace$(docker_bind_selinux_z)" -w /workspace \
    -e HOME=/tmp \
    -e "CARGO_HOME=/workspace/$CARGO_HOME_REL" \
    -e CARGO_TARGET_DIR=/workspace/target \
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
