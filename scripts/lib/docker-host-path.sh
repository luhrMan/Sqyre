#!/usr/bin/env bash
# shellcheck shell=bash
# Resolve the Docker-daemon-visible path for REPO_ROOT (docker-outside-of-docker).
#
# Nested `docker run -v "$REPO_ROOT:..."` uses the host daemon. Inside a
# devcontainer, REPO_ROOT is often /workspace, which does not exist on the host.
#
# Prefers LOCAL_WORKSPACE_FOLDER (set from ${localWorkspaceFolder} in
# .devcontainer/devcontainer.json), then docker inspect of this container's
# bind mount, then REPO_ROOT (bare-metal / CI).
#
# Requires: REPO_ROOT. Sets/exports DOCKER_HOST_REPO_ROOT.

_docker_host_path_lib="${BASH_SOURCE[0]:-$0}"

if [ -z "${REPO_ROOT:-}" ]; then
  echo "docker-host-path.sh: REPO_ROOT is not set (source repo-root.sh first)" >&2
  unset _docker_host_path_lib
  return 1 2>/dev/null || exit 1
fi

_docker_host_repo_root_resolve() {
  if [ -n "${LOCAL_WORKSPACE_FOLDER:-}" ]; then
    printf '%s\n' "$LOCAL_WORKSPACE_FOLDER"
    return 0
  fi

  # Host (or any env where the path is already daemon-visible).
  if [ ! -f /.dockerenv ] && [ ! -f /run/.containerenv ]; then
    printf '%s\n' "$REPO_ROOT"
    return 0
  fi

  if command -v docker >/dev/null 2>&1; then
    local cid src
    cid="$(hostname 2>/dev/null || true)"
    if [ -n "$cid" ]; then
      src="$(docker inspect -f \
        '{{range .Mounts}}{{if eq .Destination "'"$REPO_ROOT"'"}}{{.Source}}{{end}}{{end}}' \
        "$cid" 2>/dev/null || true)"
      if [ -z "$src" ]; then
        src="$(docker inspect -f \
          '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}' \
          "$cid" 2>/dev/null || true)"
      fi
      if [ -n "$src" ]; then
        printf '%s\n' "$src"
        return 0
      fi
    fi
  fi

  printf '%s\n' "$REPO_ROOT"
}

DOCKER_HOST_REPO_ROOT="$(_docker_host_repo_root_resolve)"
export DOCKER_HOST_REPO_ROOT

# Map a path under REPO_ROOT to the host path for docker -v / build contexts.
docker_host_path() {
  local p="${1:?docker_host_path: path required}"
  case "$p" in
    "$REPO_ROOT"|"$REPO_ROOT"/*)
      printf '%s\n' "${DOCKER_HOST_REPO_ROOT}${p#"$REPO_ROOT"}"
      ;;
    *)
      printf '%s\n' "$p"
      ;;
  esac
}

unset _docker_host_path_lib
return 0 2>/dev/null || exit 0
