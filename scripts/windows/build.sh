#!/usr/bin/env bash
# Build Sqyre for Windows (x86_64).
#
# - On Windows: native `cargo build --release` → bin/sqyre.exe
# - Elsewhere: Docker MinGW cross image → bin/sqyre.exe
#
# Env:
#   SQYRE_WINDOWS_IMAGE          image tag (default: sqyre-windows-cross:latest)
#   SQYRE_WINDOWS_FORCE_NATIVE=1 require native Windows tools (no Docker)
#   CARGO_FLAGS                  extra cargo args
#   CARGO_HOME / CARGO_TARGET_DIR  optional cache paths (docker mounts in-repo)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../lib/repo-root.sh"
# shellcheck source=scripts/lib/docker-host-path.sh
. "$SCRIPT_DIR/../lib/docker-host-path.sh"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

is_windows_host() {
  case "$(uname -s 2>/dev/null || true)" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
  esac
  [ "${OS:-}" = "Windows_NT" ]
}

need_native() {
  have_cmd cargo
}

run_native() {
  local target_dir="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
  mkdir -p "$REPO_ROOT/bin"
  echo "Building Windows release (native)…"
  (
    cd "$REPO_ROOT"
    # shellcheck disable=SC2086
    cargo build -p sqyre-app --release ${CARGO_FLAGS:-}
  )
  local src
  if [ -f "$target_dir/release/sqyre.exe" ]; then
    src="$target_dir/release/sqyre.exe"
  elif [ -f "$target_dir/x86_64-pc-windows-gnu/release/sqyre.exe" ]; then
    src="$target_dir/x86_64-pc-windows-gnu/release/sqyre.exe"
  else
    echo "Built binary not found under $target_dir" >&2
    exit 1
  fi
  cp -f "$src" "$REPO_ROOT/bin/sqyre.exe"
  echo "Windows binary: $REPO_ROOT/bin/sqyre.exe"
}

run_docker() {
  if ! have_cmd docker; then
    echo "Windows cross-build needs Docker on non-Windows hosts." >&2
    echo "Install Docker, or build on Windows with: make windows" >&2
    exit 1
  fi

  local image="${SQYRE_WINDOWS_IMAGE:-sqyre-windows-cross:latest}"
  local dockerfile="$SCRIPT_DIR/Dockerfile"
  local host_repo
  host_repo="$(docker_host_path "$REPO_ROOT")"

  if ! docker image inspect "$image" >/dev/null 2>&1; then
    echo "Building Docker image $image (one-time; compiles MinGW Tesseract)…"
    docker build -f "$dockerfile" -t "$image" "$SCRIPT_DIR"
  fi

  local cargo_home_rel=".cache/cargo"
  if [ -n "${CARGO_HOME:-}" ]; then
    case "$CARGO_HOME" in
      "$REPO_ROOT"/*) cargo_home_rel="${CARGO_HOME#"$REPO_ROOT"/}" ;;
    esac
  elif [ -x "$REPO_ROOT/.cargo-home/bin/cargo" ]; then
    cargo_home_rel=".cargo-home"
  fi
  mkdir -p "$REPO_ROOT/$cargo_home_rel" "$REPO_ROOT/target" "$REPO_ROOT/bin"

  echo "Building Windows release (docker: $image, CARGO_HOME=$cargo_home_rel)…"
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -v "$host_repo:/workspace" -w /workspace \
    -e HOME=/tmp \
    -e "CARGO_HOME=/workspace/$cargo_home_rel" \
    -e CARGO_TARGET_DIR=/workspace/target \
    -e RUSTUP_HOME=/usr/local/rustup \
    -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
    -e "CARGO_FLAGS=${CARGO_FLAGS:-}" \
    "$image" \
    bash -c 'set -euo pipefail
      cargo build -p sqyre-app --release --target x86_64-pc-windows-gnu ${CARGO_FLAGS:-}
      cp -f target/x86_64-pc-windows-gnu/release/sqyre.exe /workspace/bin/sqyre.exe
    '

  if [ ! -f "$REPO_ROOT/bin/sqyre.exe" ]; then
    echo "Docker Windows build finished but bin/sqyre.exe is missing" >&2
    exit 1
  fi
  echo "Windows binary: $REPO_ROOT/bin/sqyre.exe"
}

if [ "${SQYRE_WINDOWS_FORCE_NATIVE:-}" = "1" ]; then
  if ! is_windows_host; then
    echo "SQYRE_WINDOWS_FORCE_NATIVE=1 requires a Windows host." >&2
    exit 1
  fi
  if ! need_native; then
    echo "SQYRE_WINDOWS_FORCE_NATIVE=1 but cargo is missing." >&2
    exit 1
  fi
  run_native
elif is_windows_host; then
  run_native
else
  run_docker
fi
