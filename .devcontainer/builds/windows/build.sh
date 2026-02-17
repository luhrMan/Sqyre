#!/bin/bash
# Cross-compile Sqyre for Windows (amd64). Run from repository root.
# Builds the Docker image (MSYS2 OpenCV + Tesseract + MinGW sysroot) then
# invokes fyne-cross to produce the Windows .exe.
set -e

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

IMAGE_NAME="fyne-cross-windows:local"

echo "=== Building Windows cross-compile image (OpenCV + Tesseract + MinGW) ==="
docker build \
    -f .devcontainer/builds/windows/docker/Dockerfile.windows-amd64 \
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
echo "=== Cross-compiling Sqyre for Windows ==="
/go/bin/fyne-cross windows \
    -image "$IMAGE_NAME" \
    --app-id com.sqyre.app \
    ./cmd/sqyre

# Clean up embedded tessdata from source tree
rm -f "$TESSDATA_EMBED"

OUTPUT_DIR="$REPO_ROOT/.devcontainer/builds/windows/output"
mkdir -p "$OUTPUT_DIR"
cp -r "$REPO_ROOT/fyne-cross/bin/windows-amd64/"* "$OUTPUT_DIR/" 2>/dev/null || true
cp -r "$REPO_ROOT/fyne-cross/dist/windows-amd64/"* "$OUTPUT_DIR/" 2>/dev/null || true

# Clean up fyne-cross working directory from project root
rm -rf "$REPO_ROOT/fyne-cross"

# fyne-cross names the exe after the app name in FyneApp.toml (e.g. Sqyre.exe)
EXE_PATH="$OUTPUT_DIR/Sqyre.exe"
if [ ! -f "$EXE_PATH" ]; then
    echo "ERROR: Sqyre.exe not found in $OUTPUT_DIR" >&2
    ls -la "$OUTPUT_DIR/" 2>/dev/null
    exit 1
fi

# Patch PE SizeOfStackReserve to 16MB to avoid STATUS_STACK_OVERFLOW (0xC00000FD) on Windows
echo ""
# echo "=== Patching PE stack size (16 MB) ==="
# go run "$REPO_ROOT/.devcontainer/builds/windows/patch-pe-stack.go" "$EXE_PATH"


echo ""
echo "=== Build complete ==="
echo "Standalone exe: .devcontainer/builds/windows/output/Sqyre.exe"
