#!/usr/bin/env bash
# Regenerate README screenshots from in-memory egui tests.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export SQYRE_UPDATE_SCREENSHOTS=1
export UPDATE_SNAPSHOTS=force

mkdir -p docs/images

cargo test -p sqyre-app --test docs_screenshots

echo "Done. Commit docs/images/ if the goldens changed."
