#!/usr/bin/env bash
# Regenerate README screenshots from in-memory egui tests.
# Needs a working wgpu device (lavapipe in the dev container / CI image).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export SQYRE_UPDATE_SCREENSHOTS=1
export UPDATE_SNAPSHOTS=force

mkdir -p docs/images

cargo test -p sqyre-app --test docs_screenshots -- --nocapture

echo "Done. Commit docs/images/ if the goldens changed."
