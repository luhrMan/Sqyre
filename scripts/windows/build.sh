#!/bin/bash
# Cross-compile Sqyre for Windows (amd64). Run from repository root.
# Builds the Docker image (MSYS2 OpenCV + Tesseract + MinGW sysroot) then
# invokes fyne-cross to produce the Windows .exe.
#
# Cross-compile Windows exe. For gocv Mat profiling build, run build-matprofile.sh instead.
# Optional: EXTRA_GO_TAGS=foo ./build.sh to add build tags.
set -e

_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"
cd "$REPO_ROOT"

IMAGE_NAME="fyne-cross-windows:local"

BUILD_TAGS="customenv,gocv_specific_modules,gocv_imgproc,gocv_imgcodecs"
if [ -n "${EXTRA_GO_TAGS:-}" ]; then
    BUILD_TAGS="$BUILD_TAGS,$EXTRA_GO_TAGS"
fi

echo "=== Building Windows cross-compile image (OpenCV + Tesseract + MinGW) ==="
docker build \
    -f scripts/windows/docker/Dockerfile.windows-amd64 \
    -t "$IMAGE_NAME" \
    .

# Download tessdata for embedding into the binary via //go:embed
echo ""
echo "=== Downloading tessdata for embedding ==="
TESSDATA_EMBED="$REPO_ROOT/internal/assets/tessdata/eng.traineddata"
if [ ! -f "$TESSDATA_EMBED" ]; then
    docker run --rm \
        -v "$REPO_ROOT/internal/assets/tessdata:/out" \
        "$IMAGE_NAME" \
        bash -c 'cp /usr/local/mingw64/share/tessdata/eng.traineddata /out/'
fi
echo "  eng.traineddata ($(du -h "$TESSDATA_EMBED" | cut -f1))"

# Verify libraries exist in the Docker image before compiling
echo ""
echo "=== Verifying libraries in Docker image ==="
docker run --rm "$IMAGE_NAME" bash -c '
    echo "--- find liblept/libtesseract under /usr/local ---"
    find /usr/local -name "liblept*" -o -name "libtesseract*" 2>/dev/null | sort
    echo "--- /usr/local/lib contents ---"
    ls -la /usr/local/lib/ 2>/dev/null || echo "(empty or missing)"
    echo "--- /usr/local/mingw64/lib lept/tess ---"
    ls -la /usr/local/mingw64/lib/liblept* /usr/local/mingw64/lib/libtesseract* 2>/dev/null || echo "(none)"
'

echo ""
echo "=== Cross-compiling Sqyre for Windows (tags: $BUILD_TAGS) ==="
# Pass tags via both GOFLAGS and fyne-cross -tags so the matprofile code is included regardless of how the container runs go build.
/go/bin/fyne-cross windows \
    -image "$IMAGE_NAME" \
    -env "GOFLAGS=-tags=$BUILD_TAGS" \
    -tags "$BUILD_TAGS" \
    --app-id com.sqyre.app \
    ./cmd/sqyre

# # Clean up embedded tessdata from source tree
# rm -f "$TESSDATA_EMBED"

OUTPUT_DIR="${BIN_DIR:-$REPO_ROOT/bin}/windows-amd64"
mkdir -p "$OUTPUT_DIR"
cp -r "$REPO_ROOT/fyne-cross/bin/windows-amd64/"* "$OUTPUT_DIR/" 2>/dev/null || true
cp -r "$REPO_ROOT/fyne-cross/dist/windows-amd64/"* "$OUTPUT_DIR/" 2>/dev/null || true
# Copy run script so Windows users can double-click Run-Sqyre.cmd next to the exe
cp "$REPO_ROOT/scripts/windows/Run-Sqyre.cmd" "$OUTPUT_DIR/" 2>/dev/null || true


# fyne-cross names the exe after the app name in FyneApp.toml (e.g. Sqyre.exe)
EXE_PATH="$OUTPUT_DIR/Sqyre.exe"
if [ ! -f "$EXE_PATH" ]; then
    echo "ERROR: Sqyre.exe not found in $OUTPUT_DIR" >&2
    ls -la "$OUTPUT_DIR/" 2>/dev/null
    exit 1
fi

# Patch PE SizeOfStackReserve to 16MB to avoid STATUS_STACK_OVERFLOW (0xC00000FD) on Windows
# echo ""
# echo "=== Patching PE stack size (16 MB) ==="
# go run "$REPO_ROOT/scripts/windows/patch-pe-stack.go" "$EXE_PATH"


echo ""
echo "=== Build complete ==="
echo "Exe: $OUTPUT_DIR/Sqyre.exe"
echo "Copy output/ to Windows and run Sqyre.exe. For Mat profiling build, run build-matprofile.sh instead."
