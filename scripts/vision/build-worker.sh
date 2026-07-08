#!/usr/bin/env bash
# Build sqyre-vision with embedded ONNX models (optional bloated sidecar).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

"$ROOT/scripts/vision/download-models.sh"

OUT="${1:-$ROOT/bin/sqyre-vision}"
mkdir -p "$(dirname "$OUT")"

TAGS="detector_onnx,vision_embed"
if [[ -n "${EXTRA_GO_TAGS:-}" ]]; then
  TAGS="$TAGS,$EXTRA_GO_TAGS"
fi

echo "Building sqyre-vision -> $OUT (tags: $TAGS)"
go build -trimpath -tags="$TAGS" -o "$OUT" ./cmd/sqyre-vision

echo "Done. Place sqyre-vision next to sqyre or set vision worker path in Settings."
