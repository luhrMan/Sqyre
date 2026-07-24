#!/usr/bin/env bash
# Build Sqyre for Windows (x86_64).
#
# - On Windows: native `cargo build --release` → bin/sqyre.exe
# - Elsewhere: Docker MinGW cross image → bin/sqyre.exe
#
# Env:
#   SQYRE_WINDOWS_IMAGE            image tag (default: sqyre-windows-cross:latest)
#   SQYRE_WINDOWS_REGISTRY_IMAGE   GHCR (or other) image to pull when local tag is missing
#   SQYRE_WINDOWS_SKIP_PULL=1      never docker pull; build locally if needed
#   SQYRE_WINDOWS_FORCE_NATIVE=1   require native Windows tools (no Docker)
#   SQYRE_WINDOWS_SCCACHE=1        enable sccache (CI); default is Cargo incremental
#   CARGO_INCREMENTAL              default 1 when sccache is off. With sccache, must
#                                  stay unset (sccache rejects the var even when =0).
#   CARGO_FLAGS                    extra cargo args
#   CARGO_HOME / CARGO_TARGET_DIR  optional cache paths (docker mounts in-repo)
#   SCCACHE_DIR                    host-relative cache (default: .cache/sccache-windows)
#   SQYRE_WINDOWS_TARGET_VOLUME    Docker volume for CARGO_TARGET_DIR (auto on Win paths)
#   SQYRE_WINDOWS_CARGO_VOLUME     Docker volume for CARGO_HOME (auto on Win paths)
#   SQYRE_WINDOWS_BIND_CACHE=1     force bind-mount caches even on Windows Docker paths
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

# True when the docker daemon sees the repo on a Windows path (Docker Desktop).
# Bind-mounted target/ there makes incremental rustc I/O very slow.
is_windows_docker_bind_path() {
  case "$1" in
    [A-Za-z]:*) return 0 ;;          # C:\... or C:/...
    /mnt/[a-zA-Z]/*) return 0 ;;     # WSL /mnt/c/...
    //[a-zA-Z]/*) return 0 ;;        # //c/...
  esac
  return 1
}

# Fail early if a prior `sudo make windows` left root-owned caches (breaks incremental link).
check_target_writable() {
  local target_dir="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
  local win_dir="$target_dir/x86_64-pc-windows-gnu"
  [ -d "$win_dir" ] || return 0
  [ "$(id -u)" -eq 0 ] && return 0
  if find "$win_dir" -user root -print -quit 2>/dev/null | grep -q .; then
    echo "error: root-owned files under $win_dir (from sudo)." >&2
    echo "  Fix:  sudo chown -R \"$(id -un):$(id -gn)\" \"$target_dir\" \"$REPO_ROOT/.cache\"" >&2
    echo "  Then: rm -rf \"$win_dir/release/incremental\" && make windows" >&2
    exit 1
  fi
}

need_native() {
  have_cmd cargo
}

# ghcr.io/<owner>/<repo>-windows-cross:latest from env or git remote.
windows_registry_image() {
  if [ -n "${SQYRE_WINDOWS_REGISTRY_IMAGE:-}" ]; then
    printf '%s\n' "$SQYRE_WINDOWS_REGISTRY_IMAGE"
    return 0
  fi
  local owner="" name="" url="" path=""
  if [ -n "${GITHUB_REPOSITORY:-}" ]; then
    owner=$(printf '%s' "${GITHUB_REPOSITORY%%/*}" | tr '[:upper:]' '[:lower:]')
    name=$(printf '%s' "${GITHUB_REPOSITORY#*/}" | tr '[:upper:]' '[:lower:]')
  elif have_cmd git; then
    url="$(git -C "$REPO_ROOT" remote get-url origin 2>/dev/null || true)"
    case "$url" in
      git@github.com:*)
        path="${url#git@github.com:}"
        ;;
      https://github.com/*|http://github.com/*|ssh://git@github.com/*)
        path="${url#*github.com/}"
        path="${path#*:}"
        ;;
      *)
        path=""
        ;;
    esac
    path="${path%.git}"
    if [ -n "$path" ] && [ "${path}" = "${path#*/}" ]; then
      path=""
    fi
    if [ -n "$path" ]; then
      owner=$(printf '%s' "${path%%/*}" | tr '[:upper:]' '[:lower:]')
      name=$(printf '%s' "${path#*/}" | tr '[:upper:]' '[:lower:]')
      name="${name%%/*}"
    fi
  fi
  if [ -n "$owner" ] && [ -n "$name" ]; then
    printf 'ghcr.io/%s/%s-windows-cross:latest\n' "$owner" "$name"
    return 0
  fi
  return 1
}

cross_image_has_sccache() {
  local image="$1"
  docker run --rm --entrypoint bash "$image" -c 'command -v sccache-rustc-wrapper >/dev/null && command -v sccache >/dev/null'
}

ensure_windows_cross_image() {
  local image="$1"
  local dockerfile="$2"
  local registry=""

  if docker image inspect "$image" >/dev/null 2>&1 && cross_image_has_sccache "$image"; then
    return 0
  fi

  if registry="$(windows_registry_image)" && [ "${SQYRE_WINDOWS_SKIP_PULL:-}" != 1 ]; then
    echo "Pulling Windows cross image $registry…"
    if docker pull "$registry"; then
      docker tag "$registry" "$image"
      if cross_image_has_sccache "$image"; then
        return 0
      fi
      echo "Pulled image lacks sccache; building locally…" >&2
    else
      echo "Pull failed; building $image locally…" >&2
    fi
  fi

  echo "Building Docker image $image (MinGW Tesseract + sccache; slow once)…"
  docker build -f "$dockerfile" -t "$image" "$SCRIPT_DIR"
}

run_native() {
  local target_dir="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
  local cargo_incremental="${CARGO_INCREMENTAL:-1}"
  mkdir -p "$REPO_ROOT/bin"
  echo "Building Windows release (native, CARGO_INCREMENTAL=$cargo_incremental)…"
  (
    cd "$REPO_ROOT"
    export CARGO_INCREMENTAL="$cargo_incremental"
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
  check_target_writable
  if ! have_cmd docker; then
    echo "Windows cross-build needs Docker on non-Windows hosts." >&2
    echo "Install Docker, or build on Windows with: make windows" >&2
    exit 1
  fi

  local image="${SQYRE_WINDOWS_IMAGE:-sqyre-windows-cross:latest}"
  local dockerfile="$SCRIPT_DIR/Dockerfile"
  local host_repo
  host_repo="$(docker_host_path "$REPO_ROOT")"

  ensure_windows_cross_image "$image" "$dockerfile"

  local cargo_home_rel=".cache/cargo"
  if [ -n "${CARGO_HOME:-}" ]; then
    case "$CARGO_HOME" in
      "$REPO_ROOT"/*) cargo_home_rel="${CARGO_HOME#"$REPO_ROOT"/}" ;;
    esac
  elif [ -x "$REPO_ROOT/.cargo-home/bin/cargo" ]; then
    cargo_home_rel=".cargo-home"
  fi

  local sccache_rel=".cache/sccache-windows"
  if [ -n "${SCCACHE_DIR:-}" ]; then
    case "$SCCACHE_DIR" in
      "$REPO_ROOT"/*) sccache_rel="${SCCACHE_DIR#"$REPO_ROOT"/}" ;;
      /*) sccache_rel=".cache/sccache-windows" ;; # non-repo absolute: keep default mount path
      *) sccache_rel="$SCCACHE_DIR" ;;
    esac
  fi

  # Default: Cargo incremental (best for warm target/).
  # sccache (SQYRE_WINDOWS_SCCACHE=1) is better for CI / cold caches; it rejects
  # CARGO_INCREMENTAL when that var is set at all (even to 0).
  local use_sccache=0
  local rustc_wrapper=""
  local cargo_incremental="${CARGO_INCREMENTAL:-1}"
  local docker_incr_args=(-e "CARGO_INCREMENTAL=$cargo_incremental")
  if [ "${SQYRE_WINDOWS_SCCACHE:-0}" = "1" ]; then
    use_sccache=1
    rustc_wrapper=sccache-rustc-wrapper
    cargo_incremental=""
    docker_incr_args=()
    if [ "${CARGO_INCREMENTAL:-}" = "1" ]; then
      echo "note: CARGO_INCREMENTAL is incompatible with sccache; using sccache only" >&2
    fi
  fi

  # On Docker Desktop (Windows host path), keep cargo caches on Linux volumes —
  # incremental artifacts on a Windows bind mount are extremely slow.
  local use_linux_cache_vols=0
  if [ "${SQYRE_WINDOWS_BIND_CACHE:-0}" != "1" ] && is_windows_docker_bind_path "$host_repo"; then
    use_linux_cache_vols=1
  fi

  local cargo_home_in="/workspace/$cargo_home_rel"
  local cargo_target_in=/workspace/target
  local sccache_in="/workspace/$sccache_rel"
  local -a vol_args=()
  local target_vol="${SQYRE_WINDOWS_TARGET_VOLUME:-sqyre-windows-target}"
  local cargo_vol="${SQYRE_WINDOWS_CARGO_VOLUME:-sqyre-windows-cargo}"
  local sccache_vol="${SQYRE_WINDOWS_SCCACHE_VOLUME:-sqyre-windows-sccache}"

  mkdir -p "$REPO_ROOT/bin"
  if [ "$use_linux_cache_vols" = 1 ]; then
    docker volume create "$target_vol" >/dev/null
    docker volume create "$cargo_vol" >/dev/null
    docker volume create "$sccache_vol" >/dev/null
    cargo_home_in=/cargo-home
    cargo_target_in=/cargo-target
    sccache_in=/sccache
    vol_args+=(
      -v "$cargo_vol:/cargo-home"
      -v "$target_vol:/cargo-target"
      -v "$sccache_vol:/sccache"
    )
    # Named volumes default to root:root; the build runs as the host uid.
    docker run --rm \
      -v "$cargo_vol:/cargo-home" \
      -v "$target_vol:/cargo-target" \
      -v "$sccache_vol:/sccache" \
      "$image" \
      chown -R "$(id -u):$(id -g)" /cargo-home /cargo-target /sccache
  else
    mkdir -p "$REPO_ROOT/$cargo_home_rel" "$REPO_ROOT/$sccache_rel" "$REPO_ROOT/target"
  fi

  local cache_note=""
  if [ "$use_linux_cache_vols" = 1 ]; then
    cache_note=", linux-cache-vols"
  fi
  echo "Building Windows release (docker: $image, CARGO_HOME=$cargo_home_in, CARGO_TARGET_DIR=$cargo_target_in, incremental=${cargo_incremental:-off}, sccache=$use_sccache$cache_note)…"
  docker run --rm \
    -u "$(id -u):$(id -g)" \
    -v "$host_repo:/workspace" \
    "${vol_args[@]}" \
    -w /workspace \
    -e HOME=/tmp \
    -e "CARGO_HOME=$cargo_home_in" \
    -e "CARGO_TARGET_DIR=$cargo_target_in" \
    "${docker_incr_args[@]}" \
    -e "RUSTC_WRAPPER=$rustc_wrapper" \
    -e "SCCACHE_DIR=$sccache_in" \
    -e SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-10G}" \
    -e RUSTUP_HOME=/usr/local/rustup \
    -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
    -e "CARGO_FLAGS=${CARGO_FLAGS:-}" \
    -e "RELEASE_VERSION=${RELEASE_VERSION:-}" \
    "$image" \
    bash -c 'set -euo pipefail
      cargo build -p sqyre-app --release --target x86_64-pc-windows-gnu ${CARGO_FLAGS:-}
      cp -f "${CARGO_TARGET_DIR:-target}/x86_64-pc-windows-gnu/release/sqyre.exe" /workspace/bin/sqyre.exe
      if command -v sccache >/dev/null && [ -n "${RUSTC_WRAPPER:-}" ]; then
        sccache --show-stats || true
      fi
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
