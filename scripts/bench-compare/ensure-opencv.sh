#!/usr/bin/env bash
# Build a minimal OpenCV matching gocv v0.43 (OpenCV 4.13.0) into .cache/.
# Only core+imgproc+imgcodecs — enough for Sqyre match/OCR preprocess/find-pixel.
# Idempotent: skips work when the prefix already has opencv4.pc.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
PREFIX="${OPENCV_PREFIX:-$ROOT/.cache/opencv-$OPENCV_VERSION}"
SRC_ROOT="${OPENCV_SRC_ROOT:-$ROOT/.cache/opencv-src}"
JOBS="${OPENCV_BUILD_JOBS:-$(nproc)}"

pc="$PREFIX/lib/pkgconfig/opencv4.pc"
if [[ ! -f "$pc" && -f "$PREFIX/lib64/pkgconfig/opencv4.pc" ]]; then
  pc="$PREFIX/lib64/pkgconfig/opencv4.pc"
fi
if [[ -f "$pc" ]]; then
  # Verify version string
  ver="$(PKG_CONFIG_PATH="$(dirname "$pc")" pkg-config --modversion opencv4 2>/dev/null || true)"
  if [[ "$ver" == "$OPENCV_VERSION" ]]; then
    echo "OpenCV $OPENCV_VERSION already installed at $PREFIX"
    echo "$PREFIX"
    exit 0
  fi
fi

echo "Installing OpenCV $OPENCV_VERSION → $PREFIX (minimal: core,imgproc,imgcodecs)"

# Prefer GCC — clang as default c++ often lacks libstdc++ link flags in this image.
export CC="${CC:-gcc}"
export CXX="${CXX:-g++}"

need_pkgs=()
for p in cmake g++ pkg-config unzip curl; do
  command -v "$p" >/dev/null 2>&1 || need_pkgs+=("$p")
done
# libjpeg/png/tiff/zlib for imgcodecs; libstdc++ for the toolchain
for p in libjpeg-dev libpng-dev libtiff-dev zlib1g-dev libwebp-dev g++ build-essential; do
  dpkg -s "$p" >/dev/null 2>&1 || need_pkgs+=("$p")
done
if ((${#need_pkgs[@]})); then
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${need_pkgs[@]}"
fi

mkdir -p "$SRC_ROOT"
cd "$SRC_ROOT"

if [[ ! -d "opencv-$OPENCV_VERSION" ]]; then
  echo "Downloading opencv $OPENCV_VERSION…"
  curl -fsSL -o opencv.zip "https://github.com/opencv/opencv/archive/refs/tags/${OPENCV_VERSION}.zip"
  unzip -q opencv.zip
  rm -f opencv.zip
fi

build_dir="$SRC_ROOT/opencv-$OPENCV_VERSION/build-bench"
rm -rf "$build_dir"
mkdir -p "$build_dir"
cd "$build_dir"

cmake .. \
  -D CMAKE_BUILD_TYPE=Release \
  -D CMAKE_INSTALL_PREFIX="$PREFIX" \
  -D BUILD_SHARED_LIBS=ON \
  -D BUILD_LIST=core,imgproc,imgcodecs \
  -D BUILD_DOCS=OFF \
  -D BUILD_EXAMPLES=OFF \
  -D BUILD_TESTS=OFF \
  -D BUILD_PERF_TESTS=OFF \
  -D BUILD_opencv_apps=OFF \
  -D BUILD_opencv_java=OFF \
  -D BUILD_opencv_python=OFF \
  -D BUILD_opencv_python2=OFF \
  -D BUILD_opencv_python3=OFF \
  -D WITH_FFMPEG=OFF \
  -D WITH_GSTREAMER=OFF \
  -D WITH_GTK=OFF \
  -D WITH_QT=OFF \
  -D WITH_OPENCL=OFF \
  -D WITH_CUDA=OFF \
  -D WITH_IPP=OFF \
  -D WITH_TBB=OFF \
  -D WITH_OPENEXR=OFF \
  -D WITH_WEBP=ON \
  -D WITH_JASPER=OFF \
  -D OPENCV_GENERATE_PKGCONFIG=ON

cmake --build . --parallel "$JOBS"
cmake --install .

# Normalize pkgconfig path for callers
if [[ -f "$PREFIX/lib64/pkgconfig/opencv4.pc" && ! -f "$PREFIX/lib/pkgconfig/opencv4.pc" ]]; then
  mkdir -p "$PREFIX/lib/pkgconfig"
  cp -a "$PREFIX/lib64/pkgconfig/opencv4.pc" "$PREFIX/lib/pkgconfig/"
fi

echo "OpenCV $OPENCV_VERSION ready at $PREFIX"
echo "$PREFIX"
