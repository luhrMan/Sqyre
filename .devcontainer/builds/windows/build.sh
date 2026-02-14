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
    -f .devcontainer/builds/windows/Dockerfile.windows-amd64 \
    -t "$IMAGE_NAME" \
    .

echo ""
echo "=== Cross-compiling Sqyre for Windows ==="
/go/bin/fyne-cross windows \
    -image "$IMAGE_NAME" \
    --app-id com.sqyre.app \
    ./cmd/sqyre

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

# -----------------------------------------------------------------------
# Stage installer files
# -----------------------------------------------------------------------
WINDOWS_DIR="$REPO_ROOT/.devcontainer/builds/windows"
STAGING="$WINDOWS_DIR/staging"
rm -rf "$STAGING"
mkdir -p "$STAGING/tessdata"

echo ""
echo "=== Staging installer files ==="

# Copy the compiled executable
cp "$EXE_PATH" "$STAGING/"
echo "  Sqyre.exe"

# Collect DLL dependencies from the Docker image
echo ""
echo "=== Collecting DLL dependencies ==="
docker run --rm \
    -v "$STAGING:/staging" \
    -v "$EXE_PATH:/exe/Sqyre.exe:ro" \
    -v "$WINDOWS_DIR/collect-dlls.sh:/collect-dlls.sh:ro" \
    "$IMAGE_NAME" \
    bash /collect-dlls.sh /exe/Sqyre.exe /staging

# Copy tessdata from the Docker image
echo ""
echo "=== Extracting tessdata ==="
docker run --rm \
    -v "$STAGING/tessdata:/out" \
    "$IMAGE_NAME" \
    bash -c 'cp /usr/local/mingw64/share/tessdata/eng.traineddata /out/'
echo "  tessdata/eng.traineddata"

# -----------------------------------------------------------------------
# Build the NSIS installer
# -----------------------------------------------------------------------
echo ""
echo "=== Building Windows installer with NSIS ==="

# Read version from FyneApp.toml if present
APP_VERSION="0.5.0"
if [ -f "$REPO_ROOT/FyneApp.toml" ]; then
    VER=$(grep '^Version' "$REPO_ROOT/FyneApp.toml" | head -1 | sed 's/.*= *"//;s/".*//')
    [ -n "$VER" ] && APP_VERSION="$VER"
fi

docker run --rm \
    -v "$WINDOWS_DIR:/work" \
    -v "$OUTPUT_DIR:/output" \
    "$IMAGE_NAME" \
    makensis \
        -DAPP_VERSION="$APP_VERSION" \
        -DSTAGING_DIR=/work/staging \
        -DOUT_DIR=/output \
        /work/sqyre-installer.nsi

# Clean up staging
rm -rf "$STAGING"

echo ""
echo "=== Build complete ==="
echo "Installer: .devcontainer/builds/windows/output/SqyreSetup-${APP_VERSION}.exe"
echo "Standalone exe + DLLs also available in: .devcontainer/builds/windows/output/"
