#!/usr/bin/env bash
# preToolUse hook: seed the sandbox's per-command GOCACHE from the developer's
# real ~/.cache/go-build before any `go` command runs.
#
# Why: Cursor's shell sandbox exports a throwaway GOCACHE under
# /tmp/cursor-sandbox-cache/<hash>/go-build that is recreated (cold) for every
# command, while the real cache at ~/.cache/go-build is readable but not
# writable inside the sandbox. Without seeding, every agent `go build`/`go test`
# recompiles all CGO deps (opencv, tesseract, robotgo, gohook) from scratch.
#
# We can't override GOCACHE (the sandbox sets it as a real env var, which wins
# over `go env -w`), and we can't write into the real cache. So instead we copy
# the real cache into the sandbox's GOCACHE inside the SAME command as the go
# invocation (the dir hash changes per command, so a separate step is useless).
#
# This is a preToolUse hook because only preToolUse can rewrite the command via
# `updated_input`; beforeShellExecution cannot.
set -euo pipefail

input="$(cat)"
command="$(printf '%s' "$input" | jq -r '.tool_input.command // .command // empty')"

# Only touch invocations of the go toolchain; leave everything else untouched.
if [[ -z "$command" ]] || ! printf '%s' "$command" | grep -Eq '(^|[^[:alnum:]_./-])go[[:space:]]+(build|test|run|vet|install|generate)([[:space:]]|$)'; then
  echo '{}'
  exit 0
fi

# Best-effort seed snippet. Runs inside the command's own sandbox, so $GOCACHE
# resolves to that command's cache dir. Never fail the build if seeding fails.
seed='if [ -n "$GOCACHE" ] && [ -d "$HOME/.cache/go-build" ] && [ "${GOCACHE#/tmp/cursor-sandbox-cache/}" != "$GOCACHE" ]; then mkdir -p "$GOCACHE" && rsync -a --ignore-existing "$HOME/.cache/go-build/" "$GOCACHE/" >/dev/null 2>&1 || true; fi'

new_command="$seed
$command"

jq -n --arg cmd "$new_command" '{updated_input: {command: $cmd}}'
exit 0
