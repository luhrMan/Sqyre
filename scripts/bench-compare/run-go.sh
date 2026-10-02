#!/usr/bin/env bash
# Build and run the Go comparative harness against the historical Go tree.
#
# Vision/match sections need gocv v0.43 + OpenCV 4.13. System apt OpenCV is 4.6
# and cannot compile gocv 0.43. We provision a minimal OpenCV 4.13 under
# .cache/opencv-4.13.0 (see ensure-opencv.sh) and build with
# gocv_specific_modules so only core/imgproc/imgcodecs wrappers are linked.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
GO_ROOT="${GO_SQYRE_ROOT:-$ROOT/.cache/go-sqyre}"
GO_BIN="${GO_BIN:-}"
ITER="${BENCH_ITERATIONS:-40}"
OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
OPENCV_PREFIX="${OPENCV_PREFIX:-$ROOT/.cache/opencv-$OPENCV_VERSION}"
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

# OpenCV 4.13 InRange no longer broadcasts 1×1 bound Mats (zero hits). Overlay
# find_pixel.go to use InRangeWithScalar — real Go find-pixel path, 4.13-correct.
cp -f "$ROOT/scripts/bench-compare/go/overlays/find_pixel.go" \
  "$GO_ROOT/internal/services/find_pixel.go"

# Drop any previous stub replace so we link real gocv.
if grep -q 'gocvstub' "$GO_ROOT/go.mod" 2>/dev/null; then
  grep -v 'gocvstub\|replace gocv.io/x/gocv' "$GO_ROOT/go.mod" > "$GO_ROOT/go.mod.tmp"
  mv "$GO_ROOT/go.mod.tmp" "$GO_ROOT/go.mod"
fi
rm -rf "$GO_ROOT/third_party/gocvstub"

export GOTOOLCHAIN=local
BIN_OUT="$OUT_DIR/go-benchcompare"

# Provision OpenCV matching gocv 0.43.
bash "$ROOT/scripts/bench-compare/ensure-opencv.sh" >/dev/null
export PKG_CONFIG_PATH="$OPENCV_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
export LD_LIBRARY_PATH="$OPENCV_PREFIX/lib:${LD_LIBRARY_PATH:-}"
export CGO_ENABLED=1

# Prefer cached OpenCV over system 4.6.
if ! PKG_CONFIG_PATH="$OPENCV_PREFIX/lib/pkgconfig" pkg-config --exists opencv4; then
  echo "OpenCV $OPENCV_VERSION pkg-config not found under $OPENCV_PREFIX" >&2
  exit 1
fi
echo "Using OpenCV $(PKG_CONFIG_PATH="$OPENCV_PREFIX/lib/pkgconfig" pkg-config --modversion opencv4) from $OPENCV_PREFIX"
echo "Building Go benchcompare with real gocv (gocv_specific_modules)…"

# gocv_specific_modules excludes dnn/objdetect/video/… wrappers that need full OpenCV.
# Core+imgproc+imgcodecs (always compiled) cover MatchTemplate / InRange / preprocess.
GOCV_TAGS="gocv,gocv_specific_modules"
(
  cd "$GO_ROOT"
  # Ensure module still on gocv 0.43 (historical go.mod).
  "$GO_BIN" get gocv.io/x/gocv@v0.43.0 >/dev/null
  "$GO_BIN" build -tags "$GOCV_TAGS" -o "$BIN_OUT" ./cmd/benchcompare
)

FIXTURE="$ROOT/scripts/bench-compare/fixtures/db.yaml"
# Run from the Go worktree so git_rev in the report matches the historical tree.
(
  cd "$GO_ROOT"
  export LD_LIBRARY_PATH="$OPENCV_PREFIX/lib:${LD_LIBRARY_PATH:-}"
  "$BIN_OUT" -json -iterations "$ITER" -fixture-db "$FIXTURE" \
    | tee "$OUT_DIR/go-latest.json" >/dev/null
  "$BIN_OUT" -iterations "$ITER" -fixture-db "$FIXTURE" \
    | tee "$OUT_DIR/go-latest.txt"
)
echo "Wrote $OUT_DIR/go-latest.json"
