#!/bin/bash
# Cross-compile Sqyre for Android using fyne-cross.
# Builds APKs for: arm64, arm (ARMv7), amd64, 386.
# Run from repository root. Requires Docker and fyne-cross (go install github.com/fyne-io/fyne-cross@latest).
#
# First build the custom image (Go 1.26; base image has 1.24):
#   docker build -f .devcontainer/builds/android/docker/Dockerfile.android -t fyne-cross-android:local .
# Note: desktop-only deps (e.g. gocv, robotgo) may require build tags for Android.
set -e

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

# Ensure tessdata for embedding (go:embed in internal/assets/tessdata.go)
TESSDATA_EMBED="$REPO_ROOT/internal/assets/tessdata/eng.traineddata"
if [ ! -f "$TESSDATA_EMBED" ]; then
  echo "=== Downloading tessdata for embedding ==="
  mkdir -p "$(dirname "$TESSDATA_EMBED")"
  curl -sSL -o "$TESSDATA_EMBED" \
    https://github.com/tesseract-ocr/tessdata/raw/main/eng.traineddata
  echo "  eng.traineddata ($(du -h "$TESSDATA_EMBED" | cut -f1))"
fi

IMAGE_NAME="${FYNE_CROSS_ANDROID_IMAGE:-fyne-cross-android:local}"
# Android architectures supported by fyne-cross: arm64, arm, amd64, 386
ANDROID_ARCHES="${ANDROID_ARCHES:-arm64,arm,amd64,386}"

echo "=== Cross-compiling Sqyre for Android (arch: $ANDROID_ARCHES) ==="
fyne-cross android \
  -image "$IMAGE_NAME" \
  --arch "$ANDROID_ARCHES" \
  --app-id com.sqyre.app \
  ./cmd/sqyre

OUTPUT_DIR="$REPO_ROOT/.devcontainer/builds/android/output"
mkdir -p "$OUTPUT_DIR"
cp -r "$REPO_ROOT/fyne-cross/dist/android-"* "$OUTPUT_DIR/" 2>/dev/null || true

echo ""
echo "=== Build complete ==="
echo "APKs in: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR/" 2>/dev/null || true
