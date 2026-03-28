#!/usr/bin/env bash
# shellcheck shell=bash
# Resolve repository root and set REPO_ROOT (exported).
#
# Source from a project script, e.g.:
#   _here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   . "$_here/../lib/repo-root.sh"
#
# Resolution: walk upward from this file's directory until go.mod is found; if that
# fails, try git rev-parse --show-toplevel from this file's directory.

_repo_root_lib="${BASH_SOURCE[0]:-$0}"
_lib_dir="$(cd "$(dirname "$_repo_root_lib")" && pwd)"
d="$_lib_dir"

while [[ "$d" != "/" && ! -f "$d/go.mod" ]]; do
  d="$(dirname "$d")"
done

if [[ ! -f "$d/go.mod" ]]; then
  _git_root="$(git -C "$_lib_dir" rev-parse --show-toplevel 2>/dev/null)" || true
  if [[ -n "${_git_root}" && -f "${_git_root}/go.mod" ]]; then
    REPO_ROOT="${_git_root}"
    export REPO_ROOT
    unset _repo_root_lib _lib_dir d _git_root
    return 0 2>/dev/null || exit 0
  fi
  echo "repo-root.sh: could not find repo root (no go.mod above ${_lib_dir}; git fallback failed)" >&2
  unset _repo_root_lib _lib_dir d _git_root
  return 1 2>/dev/null || exit 1
fi

REPO_ROOT="$d"
export REPO_ROOT
unset _repo_root_lib _lib_dir d
return 0 2>/dev/null || exit 0
