#!/usr/bin/env bash
# Build sqyre-vision without embedded models (uses ~/.sqyre/models at runtime).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

"$ROOT/scripts/vision/download-clip-tokenizer.sh"

OUT="${1:-$ROOT/bin/sqyre-vision}"
mkdir -p "$(dirname "$OUT")"

TAGS="detector_onnx"
if [[ -n "${EXTRA_GO_TAGS:-}" ]]; then
  TAGS="$TAGS,$EXTRA_GO_TAGS"
fi

go build -trimpath -tags="$TAGS" -o "$OUT" ./cmd/sqyre-vision
echo "Built lean worker: $OUT"
echo "Run make vision-models (downloads ONNX + libonnxruntime 1.26.0; caches built on first load)."
