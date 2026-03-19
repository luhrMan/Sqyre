#!/bin/bash
# Build static OpenCV for MinGW x86_64. Called from build-static-libs.sh.
# Expects: DESTDIR ($1), BUILD_DIR (env, default /tmp/staticbuild), CC, CXX, CMAKE_CROSS, PKG_CONFIG_PATH.
# Skips build if a previous install for this OPENCV_VERSION already exists in DESTDIR.
set -e
set -x

DESTDIR="${1:?Usage: build-opencv-windows.sh DESTDIR [BUILD_DIR]}"
BUILD_DIR="${2:-/tmp/staticbuild}"
OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
VERSION_MARKER="$DESTDIR/.opencv-version"

# Ensure 4130 symlinks exist (idempotent)
ensure_symlinks() {
    cd "$DESTDIR/lib"
    for f in libopencv_*.a; do
        [ -f "$f" ] || continue
        base="${f%.a}"
        base="${base#lib}"
        [ -e "lib${base}4130.a" ] || ln -sf "$f" "lib${base}4130.a"
    done
}

# Skip if a matching build already exists (version rarely changes)
if [ -f "$VERSION_MARKER" ] && [ "$(cat "$VERSION_MARKER")" = "$OPENCV_VERSION" ]; then
    if [ -f "$DESTDIR/lib/libopencv_core.a" ]; then
        echo "=== OpenCV ${OPENCV_VERSION} already built in $DESTDIR (skipping) ==="
        ensure_symlinks
        exit 0
    fi
fi

cd "$BUILD_DIR"
echo "=== Building OpenCV ${OPENCV_VERSION} (Windows static) ==="
if [ ! -d "opencv-${OPENCV_VERSION}" ]; then
    curl -f -sL "https://github.com/opencv/opencv/archive/refs/tags/${OPENCV_VERSION}.tar.gz" -o opencv.tar.gz && tar xzf opencv.tar.gz
    curl -f -sL "https://github.com/opencv/opencv_contrib/archive/refs/tags/${OPENCV_VERSION}.tar.gz" -o opencv_contrib.tar.gz && tar xzf opencv_contrib.tar.gz
fi
cd "opencv-${OPENCV_VERSION}"
mkdir -p build && cd build
cmake .. $CMAKE_CROSS \
  -DCMAKE_INSTALL_PREFIX="$DESTDIR" \
  -DBUILD_SHARED_LIBS=OFF -DBUILD_opencv_apps=OFF -DBUILD_EXAMPLES=OFF -DBUILD_TESTS=OFF -DBUILD_PERF_TESTS=OFF \
  -DWITH_QT=OFF -DWITH_GTK=OFF -DWITH_IPP=OFF -DWITH_OPENCL=OFF -DWITH_CUDA=OFF \
  -DWITH_TIFF=OFF -DWITH_WEBP=OFF -DWITH_OPENJPEG=OFF -DWITH_OPENEXR=OFF -DWITH_JASPER=OFF \
  -DWITH_DSHOW=OFF -DWITH_MSMF=OFF -DWITH_VFW=OFF \
  -DBUILD_ZLIB=OFF -DBUILD_PNG=OFF -DBUILD_JPEG=OFF \
  -DZLIB_ROOT="$DESTDIR" \
  -DPNG_PNG_INCLUDE_DIR="$DESTDIR/include" -DPNG_LIBRARY_RELEASE="$DESTDIR/lib/libpng16.a" \
  -DJPEG_INCLUDE_DIR="$DESTDIR/include" -DJPEG_LIBRARY="$DESTDIR/lib/libjpeg.a" \
  -DOPENCV_GENERATE_PKGCONFIG=ON \
  -DOPENCV_EXTRA_MODULES_PATH="../../opencv_contrib-${OPENCV_VERSION}/modules"
make -j$(nproc)
make install
cd ../..

echo "$OPENCV_VERSION" > "$VERSION_MARKER"
ensure_symlinks
cd "$BUILD_DIR"
echo "=== OpenCV Windows static build complete ==="
