#!/bin/bash
# Build OpenCV from source for Linux (native), used by the main devcontainer.
# Build and install live under OPENCV_ROOT (default /opt/opencv/linux).
# Skips build if a previous install for this OPENCV_VERSION already exists.
set -e

OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
OPENCV_ROOT="${OPENCV_ROOT:-/opt/opencv/linux}"
BUILD_DIR="$OPENCV_ROOT/build"
INSTALL_PREFIX="$OPENCV_ROOT/install"
VERSION_MARKER="$OPENCV_ROOT/.opencv-version"

# Skip if a matching build already exists (version rarely changes)
if [ -f "$VERSION_MARKER" ] && [ "$(cat "$VERSION_MARKER")" = "$OPENCV_VERSION" ] && [ -d "$INSTALL_PREFIX/lib" ]; then
    echo "=== OpenCV ${OPENCV_VERSION} already built at $INSTALL_PREFIX (skipping) ==="
    echo "$INSTALL_PREFIX/lib" > /etc/ld.so.conf.d/opencv-linux.conf 2>/dev/null || true
    ldconfig 2>/dev/null || true
    exit 0
fi

echo "=== Building OpenCV ${OPENCV_VERSION} for Linux ==="
echo "  Build/install under: $OPENCV_ROOT"
mkdir -p "$BUILD_DIR"
if [ ! -d "$OPENCV_ROOT/opencv-${OPENCV_VERSION}" ]; then
    curl -sSL "https://github.com/opencv/opencv/archive/${OPENCV_VERSION}.tar.gz" -o "$OPENCV_ROOT/opencv.tgz"
    tar -xzf "$OPENCV_ROOT/opencv.tgz" -C "$OPENCV_ROOT" && rm "$OPENCV_ROOT/opencv.tgz"
fi
cd "$BUILD_DIR"
if [ ! -f "$BUILD_DIR/CMakeCache.txt" ]; then
    cmake "$OPENCV_ROOT/opencv-${OPENCV_VERSION}" \
      -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX="$INSTALL_PREFIX" \
      -DBUILD_SHARED_LIBS=ON \
      -DOPENCV_GENERATE_PKGCONFIG=ON \
      -DBUILD_LIST=core,imgproc,imgcodecs,videoio,highgui,video,calib3d,features2d,objdetect,dnn,photo \
      -DBUILD_TESTS=OFF \
      -DBUILD_PERF_TESTS=OFF \
      -DBUILD_EXAMPLES=OFF \
      -DBUILD_opencv_apps=OFF \
      -DWITH_JPEG=ON \
      -DWITH_PNG=ON \
      -DWITH_TIFF=ON \
      -DWITH_WEBP=ON \
      -DWITH_OPENJPEG=ON \
      -DWITH_GTK=OFF \
      -DWITH_QT=OFF
fi
ninja -j"$(nproc)" && ninja install
echo "$OPENCV_VERSION" > "$VERSION_MARKER"
# Register install lib path for runtime linker
echo "$INSTALL_PREFIX/lib" > /etc/ld.so.conf.d/opencv-linux.conf && ldconfig
# Optional: remove source and build tree to keep image size down (set OPENCV_KEEP_SOURCE=1 to keep)
if [ "${OPENCV_KEEP_SOURCE:-0}" != "1" ]; then
    rm -rf "$OPENCV_ROOT/opencv-${OPENCV_VERSION}" "$BUILD_DIR"
fi
echo "=== OpenCV for Linux build complete (installed to $INSTALL_PREFIX) ==="
