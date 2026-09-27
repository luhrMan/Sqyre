#!/usr/bin/env bash
# Build and run the Go comparative harness against the historical Go tree.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
GO_ROOT="${GO_SQYRE_ROOT:-$ROOT/.cache/go-sqyre}"
GO_BIN="${GO_BIN:-}"
ITER="${BENCH_ITERATIONS:-40}"
mkdir -p "$OUT_DIR"

if [[ -z "$GO_BIN" ]]; then
  if [[ -x /home/ubuntu/sdk/go1.26.0/bin/go ]]; then
    GO_BIN=/home/ubuntu/sdk/go1.26.0/bin/go
  elif command -v go >/dev/null 2>&1; then
    GO_BIN="$(command -v go)"
  else
    echo "Go toolchain not found. Install Go ≥ 1.22 (module wants 1.26)." >&2
    exit 1
  fi
fi

"$ROOT/scripts/bench-compare/fetch-go-tree.sh"

# Overlay bench cmd + tiny export helpers into the worktree (not committed there).
rm -rf "$GO_ROOT/cmd/benchcompare"
mkdir -p "$GO_ROOT/cmd/benchcompare"
cp -a "$ROOT/scripts/bench-compare/go/cmd/benchcompare/." "$GO_ROOT/cmd/benchcompare/"

# Embedded eng.traineddata is required to compile internal/assets (vision/OCR deps).
TESSDATA_SRC="$ROOT/assets/tessdata/eng.traineddata"
if [[ ! -f "$TESSDATA_SRC" ]]; then
  bash "$ROOT/scripts/download-tessdata.sh"
fi
mkdir -p "$GO_ROOT/internal/assets/tessdata"
cp -f "$TESSDATA_SRC" "$GO_ROOT/internal/assets/tessdata/eng.traineddata"

# Export findPixel for the harness without permanently patching history.
cat > "$GO_ROOT/internal/services/bench_export.go" <<'EOF'
package services

import "image"

// FindPixelInRGBAForBench exposes findPixelInRGBA for scripts/bench-compare.
func FindPixelInRGBAForBench(rgba *image.RGBA, tr, tg, tb uint8, tolerance int) (int, int, bool) {
	return findPixelInRGBA(rgba, tr, tg, tb, tolerance)
}
EOF

export GOTOOLCHAIN=local
BIN_OUT="$OUT_DIR/go-benchcompare"

use_gocv_stub() {
  # Historical models.Program only needs *gocv.Mat; real gocv 0.43 needs newer OpenCV
  # than Ubuntu 4.6. Stub lets yaml/codec sections build; vision sections stay skipped.
  echo "Using gocv stub (yaml/codec sections only; match/OCR skipped)." >&2
  rm -rf "$GO_ROOT/third_party/gocvstub"
  mkdir -p "$GO_ROOT/third_party"
  cp -a "$ROOT/scripts/bench-compare/go/gocvstub" "$GO_ROOT/third_party/gocvstub"
  if ! grep -q 'gocvstub' "$GO_ROOT/go.mod"; then
    printf '\nreplace gocv.io/x/gocv => ./third_party/gocvstub\n' >> "$GO_ROOT/go.mod"
  fi
}

echo "Building Go benchcompare…"
built=0
if pkg-config --exists opencv4 2>/dev/null || pkg-config --exists opencv 2>/dev/null; then
  echo "OpenCV detected — attempting real gocv build…"
  if (cd "$GO_ROOT" && "$GO_BIN" build -tags gocv -o "$BIN_OUT" ./cmd/benchcompare); then
    built=1
  else
    echo "Real gocv build failed (OpenCV/gocv mismatch is common)." >&2
  fi
fi

if [[ "$built" -ne 1 ]]; then
  use_gocv_stub
  (cd "$GO_ROOT" && "$GO_BIN" build -o "$BIN_OUT" ./cmd/benchcompare)
fi

FIXTURE="$ROOT/scripts/bench-compare/fixtures/db.yaml"
# Run from the Go worktree so git_rev in the report matches the historical tree.
(
  cd "$GO_ROOT"
  "$BIN_OUT" -json -iterations "$ITER" -fixture-db "$FIXTURE" \
    | tee "$OUT_DIR/go-latest.json" >/dev/null
  "$BIN_OUT" -iterations "$ITER" -fixture-db "$FIXTURE" \
    | tee "$OUT_DIR/go-latest.txt"
)
echo "Wrote $OUT_DIR/go-latest.json"
