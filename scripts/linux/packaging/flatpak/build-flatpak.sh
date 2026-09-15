#!/usr/bin/env bash
# Build Sqyre Flatpak (Flathub-style manifest under this directory).
#
# Prefer native flatpak-builder when tools + bubblewrap user namespaces work.
# Otherwise re-run inside a privileged Flatpak builder container when Docker
# is available (typical Devcontainer: tools installed, but userns blocked).
#
# Output: bin/com.sqyre.app.flatpak (single-file bundle)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../../lib/repo-root.sh"
# shellcheck source=scripts/lib/docker-host-path.sh
. "$SCRIPT_DIR/../../../lib/docker-host-path.sh"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

APP_ID="com.sqyre.app"
MANIFEST_SRC="$SCRIPT_DIR/${APP_ID}.yml"
BUILD_DIR="$SCRIPT_DIR/build-dir"
REPO_DIR="$SCRIPT_DIR/repo"
STATE_DIR="$SCRIPT_DIR/.flatpak-builder"

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
  have_cmd flatpak-builder && have_cmd flatpak && have_cmd ostree
}

# flatpak-builder needs bubblewrap user namespaces. Devcontainers often block
# unprivileged userns even when flatpak-builder is installed.
can_bwrap_userns() {
  have_cmd bwrap || return 1
  bwrap --bind / / --dev /dev true >/dev/null 2>&1
}

# Docker Flatpak builds often create state/build/repo as root. Native rebuilds
# then fail writing ccache.conf ("Permission denied"). Reclaim ownership.
ensure_writable_packaging_dirs() {
  local d
  for d in "$STATE_DIR" "$BUILD_DIR" "$REPO_DIR"; do
    mkdir -p "$d"
    if [ -w "$d" ] && [ -z "$(find "$d" ! -user "$(id -u)" -print -quit 2>/dev/null)" ]; then
      continue
    fi
    echo "Fixing ownership of $d (root-owned leftovers from Docker Flatpak builds)…"
    if have_cmd sudo; then
      sudo chown -R "$(id -u):$(id -g)" "$d"
    else
      echo "ERROR: $d is not writable by $(id -un). Run:" >&2
      echo "  sudo chown -R $(id -u):$(id -g) $d" >&2
      exit 1
    fi
  done
}

ensure_tessdata() {
  if [ ! -f "$REPO_ROOT/assets/tessdata/eng.traineddata" ]; then
    echo "Downloading eng.traineddata…"
    "$REPO_ROOT/scripts/download-tessdata.sh"
  fi
}

ensure_icon() {
  ICON="$REPO_ROOT/crates/sqyre-app/assets/icons/sqyre.png"
  if [ ! -f "$ICON" ]; then
    "$SCRIPT_DIR/../generate-app-icon.sh" 256
  fi
}

ensure_cargo_sources() {
  if [ ! -f "$SCRIPT_DIR/cargo-sources.json" ]; then
    echo "cargo-sources.json missing; generating…"
    "$SCRIPT_DIR/generate-cargo-sources.sh"
  fi
}

ensure_runtimes() {
  if ! flatpak remotes --user 2>/dev/null | grep -q flathub; then
    flatpak remote-add --user --if-not-exists flathub \
      https://flathub.org/repo/flathub.flatpakrepo
  fi
  flatpak install -y --user flathub \
    org.freedesktop.Platform//25.08 \
    org.freedesktop.Sdk//25.08 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08 \
    org.freedesktop.Sdk.Extension.llvm21//25.08
}

run_native() {
  ensure_tessdata
  ensure_icon
  ensure_cargo_sources
  ensure_writable_packaging_dirs
  ensure_runtimes

  RECIPE_TMP="$(mktemp -p "$SCRIPT_DIR" .com.sqyre.app.XXXXXX.yml)"
  cleanup() { rm -f "$RECIPE_TMP"; }
  trap cleanup EXIT

  sed -e "s#__APP_VERSION__#${APP_VERSION}#g" "$MANIFEST_SRC" > "$RECIPE_TMP"

  OUT_DIR="$REPO_ROOT/bin"
  mkdir -p "$OUT_DIR"
  BUNDLE="$OUT_DIR/${APP_ID}.flatpak"

  echo "Building Flatpak v${APP_VERSION} (native flatpak-builder)…"
  # Build from repo root so type:dir path ../../../.. resolves correctly
  # when SOURCE_DIR is the recipe parent (flatpak/).
  #
  # --disable-rofiles-fuse: required when /dev/fuse is missing (typical
  # Devcontainer / nested Docker). Without it flatpak-builder fails with
  # "fuse: device not found" / "rofiles … not initialized".
  (
    cd "$REPO_ROOT"
    flatpak-builder \
      --user \
      --force-clean \
      --disable-rofiles-fuse \
      --install-deps-from=flathub \
      --state-dir="$STATE_DIR" \
      --repo="$REPO_DIR" \
      "$BUILD_DIR" \
      "$RECIPE_TMP"
  )

  echo "Exporting bundle $BUNDLE…"
  flatpak build-bundle "$REPO_DIR" "$BUNDLE" "$APP_ID" --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
  echo "Flatpak bundle: $BUNDLE"
  echo "Install: flatpak install --user $BUNDLE"
  echo "Probe:   flatpak run --command=sqyre-probe $APP_ID --json"
}

run_docker() {
  if ! have_cmd docker; then
    echo "flatpak-builder missing and Docker not available." >&2
    echo "Install flatpak + flatpak-builder, or Docker for the fallback build." >&2
    exit 1
  fi

  # Privileged Flathub CI image (bwrap + flatpak-builder).
  IMAGE="${SQYRE_FLATPAK_IMAGE:-ghcr.io/flathub-infra/flatpak-github-actions:freedesktop-25.08}"

  ensure_tessdata
  ensure_icon
  ensure_cargo_sources

  mkdir -p "$REPO_ROOT/bin" "$STATE_DIR" "$BUILD_DIR" "$REPO_DIR"

  echo "Building Flatpak v${APP_VERSION} (docker: $IMAGE)…"
  # Match host UID so .flatpak-builder / build-dir / repo stay writable for
  # native rebuilds (root-owned ccache.conf otherwise Permission denied).
  docker run --rm --privileged \
    -u "$(id -u):$(id -g)" \
    -v "$(docker_host_path "$REPO_ROOT"):/workspace$(docker_bind_selinux_z)" \
    -w /workspace \
    -e HOME=/tmp \
    -e RELEASE_VERSION="$APP_VERSION" \
    -e SQYRE_FLATPAK_FORCE_NATIVE=1 \
    "$IMAGE" \
    bash -c 'set -euo pipefail
      # Image ships flatpak-builder; ensure flathub remote for the build user.
      flatpak remote-add --user --if-not-exists flathub \
        https://flathub.org/repo/flathub.flatpakrepo || true
      scripts/linux/packaging/flatpak/build-flatpak.sh'

  # If the image ignored -u (or nested tools wrote as root), reclaim ownership.
  ensure_writable_packaging_dirs

  OUT="$REPO_ROOT/bin/${APP_ID}.flatpak"
  if [ ! -f "$OUT" ]; then
    echo "Docker Flatpak build finished but $OUT is missing" >&2
    exit 1
  fi
  echo "Flatpak bundle: $OUT"
}

# Prefer native when tools + bwrap userns work. Otherwise privileged Docker
# (SQYRE_FLATPAK_FORCE_NATIVE=1 inside that container to avoid recursion).
if [ "${SQYRE_FLATPAK_FORCE_NATIVE:-}" = "1" ]; then
  if ! need_native_tools; then
    echo "SQYRE_FLATPAK_FORCE_NATIVE=1 but required tools are missing." >&2
    echo "Need: flatpak-builder, flatpak, ostree" >&2
    exit 1
  fi
  if ! can_bwrap_userns; then
    echo "SQYRE_FLATPAK_FORCE_NATIVE=1 but bubblewrap cannot create user namespaces." >&2
    echo "Host/kernel must allow userns (or run via privileged Docker without FORCE_NATIVE)." >&2
    exit 1
  fi
  run_native
elif need_native_tools && can_bwrap_userns; then
  run_native
elif have_cmd docker; then
  if need_native_tools && ! can_bwrap_userns; then
    echo "Native flatpak-builder found, but bubblewrap cannot create user namespaces."
    echo "Using privileged Docker fallback…"
  else
    echo "Native Flatpak tools not found; using Docker fallback…"
  fi
  run_docker
else
  echo "Cannot build Flatpak: need flatpak-builder+bwrap userns, or Docker." >&2
  echo "In a Devcontainer, Docker-outside-of-Docker + privileged fallback is required." >&2
  exit 1
fi
