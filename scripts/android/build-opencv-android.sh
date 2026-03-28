#!/bin/bash
# Build OpenCV (and optional contrib) for Android ABIs.
# Adapted from https://gist.github.com/ogero/c19458cf64bd3e91faae85c3ac887481
# Uses NDK CMake toolchain on Linux (no MinGW). Produces shared libs per ABI.
set -e

OPENCV_VERSION="${OPENCV_VERSION:-4.10.0}"
CONTRIB_VERSION="${CONTRIB_VERSION:-4.10.0}"
ANDROID_NDK="${ANDROID_NDK:-/opt/android-ndk}"
ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-21}"
BUILD_ROOT="${BUILD_ROOT:-/opt/opencv/android}"
# ABIs: armeabi-v7a (arm), arm64-v8a (arm64), x86, x86_64
ABIS="${ABIS:-armeabi-v7a,arm64-v8a,x86,x86_64}"
USE_CONTRIB="${USE_CONTRIB:-1}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$BUILD_ROOT/opencv-${OPENCV_VERSION}"
CONTRIB_DIR="$BUILD_ROOT/opencv_contrib-${CONTRIB_VERSION}"
TOOLCHAIN_FILE="$ANDROID_NDK/build/cmake/android.toolchain.cmake"

if [ ! -f "$TOOLCHAIN_FILE" ]; then
    echo "ERROR: Android NDK toolchain not found: $TOOLCHAIN_FILE"
    echo "Set ANDROID_NDK to your NDK root (e.g. /opt/android-ndk)"
    exit 1
fi

echo "=== OpenCV $OPENCV_VERSION for Android ==="
echo "  NDK: $ANDROID_NDK"
echo "  Build root: $BUILD_ROOT"
echo "  Contrib: $USE_CONTRIB"

VERSION_MARKER="$BUILD_ROOT/.opencv-version"
INSTALL_SDK="$BUILD_ROOT/opencv-android-sdk"
# Skip if a matching build already exists (version rarely changes)
if [ -f "$VERSION_MARKER" ] && [ "$(cat "$VERSION_MARKER")" = "$OPENCV_VERSION" ]; then
    FIRST_ABI="${ABIS%%,*}"
    if [ -d "$INSTALL_SDK/native/libs/$FIRST_ABI" ] && [ -n "$(ls -A "$INSTALL_SDK/native/libs/$FIRST_ABI" 2>/dev/null)" ]; then
        echo "  OpenCV $OPENCV_VERSION already built at $BUILD_ROOT (skipping)"
        ls -la "$INSTALL_SDK/native/libs/" 2>/dev/null || true
        exit 0
    fi
fi

mkdir -p "$BUILD_ROOT"
cd "$BUILD_ROOT"

# Download OpenCV source if missing
if [ ! -d "$SRC_DIR" ]; then
    echo "=== Downloading OpenCV $OPENCV_VERSION ==="
    curl -sL "https://github.com/opencv/opencv/archive/refs/tags/${OPENCV_VERSION}.tar.gz" -o opencv.tar.gz
    tar xzf opencv.tar.gz
    rm opencv.tar.gz
fi

# Download opencv_contrib if requested
if [ "$USE_CONTRIB" = "1" ] && [ ! -d "$CONTRIB_DIR" ]; then
    echo "=== Downloading opencv_contrib $CONTRIB_VERSION ==="
    curl -sL "https://github.com/opencv/opencv_contrib/archive/refs/tags/${CONTRIB_VERSION}.tar.gz" -o contrib.tar.gz
    tar xzf contrib.tar.gz
    rm contrib.tar.gz
fi

CONTRIB_CMAKE=""
if [ "$USE_CONTRIB" = "1" ] && [ -d "$CONTRIB_DIR" ]; then
    CONTRIB_CMAKE="-DOPENCV_EXTRA_MODULES_PATH=$CONTRIB_DIR/modules"
fi

# Build each ABI (gist: one build dir per ABI, shared libs, no Java)
for ABI in $(echo "$ABIS" | tr ',' ' '); do
    BUILD_DIR="$BUILD_ROOT/build_${ABI}"
    INSTALL_DIR="$BUILD_ROOT/install_${ABI}"
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"

    echo ""
    echo "=== Configuring OpenCV for $ABI ==="
    cmake -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN_FILE" \
        -DANDROID_NDK="$ANDROID_NDK" \
        -DANDROID_ABI="$ABI" \
        -DANDROID_NATIVE_API_LEVEL="$ANDROID_API_LEVEL" \
        -DANDROID_STL=c++_shared \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
        -DBUILD_SHARED_LIBS=ON \
        -DBUILD_STATIC_LIBS=OFF \
        -DBUILD_ANDROID_EXAMPLES=OFF \
        -DBUILD_DOCS=OFF \
        -DBUILD_TESTS=OFF \
        -DBUILD_PERF_TESTS=OFF \
        -DBUILD_JAVA=OFF \
        -DBUILD_opencv_apps=OFF \
        -DENABLE_PRECOMPILED_HEADERS=OFF \
        -DWITH_CAROTENE=OFF \
        -DBUILD_ZLIB=ON \
        $CONTRIB_CMAKE \
        "$SRC_DIR"

    echo "=== Building $ABI ==="
    ninja -j"$(nproc)"
    ninja install

    echo "  Installed to $INSTALL_DIR"
done

# Consolidate installs into a single sdk-like layout (optional)
mkdir -p "$INSTALL_SDK/native/libs"
mkdir -p "$INSTALL_SDK/native/jni/include"
for ABI in $(echo "$ABIS" | tr ',' ' '); do
    INSTALL_DIR="$BUILD_ROOT/install_${ABI}"
    if [ -d "$INSTALL_DIR/lib" ]; then
        cp -a "$INSTALL_DIR/lib" "$INSTALL_SDK/native/libs/$ABI" 2>/dev/null || true
    fi
    if [ -d "$INSTALL_DIR/include" ] && [ ! -L "$INSTALL_SDK/native/jni/include/opencv2" ]; then
        cp -a "$INSTALL_DIR/include/"* "$INSTALL_SDK/native/jni/include/" 2>/dev/null || true
    fi
done

echo "$OPENCV_VERSION" > "$VERSION_MARKER"
echo ""
echo "=== OpenCV for Android build complete ==="
echo "  Per-ABI installs: $BUILD_ROOT/install_<abi>"
echo "  SDK-style layout:  $INSTALL_SDK/native/libs/<abi> and .../jni/include"
ls -la "$INSTALL_SDK/native/libs/" 2>/dev/null || true
