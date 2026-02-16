#!/bin/bash
# Build static OpenCV and Tesseract (and deps) for MinGW x86_64.
# Run inside a container with gcc-mingw-w64-x86-64, g++-mingw-w64-x86-64, cmake.
# Installs to DESTDIR (e.g. /usr/local/mingw64-static).
set -e
set -x

DESTDIR="${1:-/usr/local/mingw64-static}"
BUILD_DIR=/tmp/staticbuild
export PKG_CONFIG_PATH="$DESTDIR/lib/pkgconfig"
export PATH="$DESTDIR/bin:$PATH"

CC=x86_64-w64-mingw32-gcc-posix
CXX=x86_64-w64-mingw32-g++-posix
export CC CXX

# Prefer -posix; fall back to non-posix if not present
command -v "$CC" >/dev/null 2>&1 || CC=x86_64-w64-mingw32-gcc
command -v "$CXX" >/dev/null 2>&1 || CXX=x86_64-w64-mingw32-g++
export CC CXX

mkdir -p "$DESTDIR" "$BUILD_DIR"
cd "$BUILD_DIR"

# ----- zlib -----
echo "=== Building zlib ==="
curl -f -sL https://zlib.net/zlib-1.3.1.tar.gz -o zlib.tar.gz && tar xzf zlib.tar.gz
cd zlib-1.3.1
cmake -B build -G "Unix Makefiles" \
  -DCMAKE_SYSTEM_NAME=Windows -DCMAKE_C_COMPILER="$CC" \
  -DCMAKE_INSTALL_PREFIX="$DESTDIR" -DZLIB_BUILD_SHARED=OFF -DZLIB_BUILD_STATIC=ON
cmake --build build -j$(nproc)
cmake --install build
# Many dependents (libpng, etc.) look for -lz; zlib installs as libzlibstatic.a
ln -sf libzlibstatic.a "$DESTDIR/lib/libz.a"
cd ..

# ----- libpng -----
echo "=== Building libpng ==="
curl -f -sL https://sourceforge.net/projects/libpng/files/libpng16/1.6.43/libpng-1.6.43.tar.gz/download -o libpng.tar.gz
tar xzf libpng.tar.gz
cd libpng-1.6.43
export CPPFLAGS="-I$DESTDIR/include"
export LDFLAGS="-L$DESTDIR/lib"
export CC CXX
./configure --host=x86_64-w64-mingw32 --prefix="$DESTDIR" --enable-static --disable-shared
make -j$(nproc)
make install
cd ..

# CPPFLAGS/LDFLAGS were only needed for libpng's autotools configure.
# Unset them so they don't leak into CMake-based builds and corrupt try_compile.
unset CPPFLAGS LDFLAGS

# ----- libjpeg-turbo -----
echo "=== Building libjpeg-turbo ==="
curl -f -sL https://github.com/libjpeg-turbo/libjpeg-turbo/archive/refs/tags/2.1.5.1.tar.gz -o jpeg.tar.gz && tar xzf jpeg.tar.gz
cd libjpeg-turbo-2.1.5.1
cmake -B build -G "Unix Makefiles" \
  -DCMAKE_SYSTEM_NAME=Windows -DCMAKE_SYSTEM_PROCESSOR=x86_64 \
  -DCMAKE_C_COMPILER="$CC" -DCMAKE_INSTALL_PREFIX="$DESTDIR" \
  -DENABLE_SHARED=OFF -DENABLE_STATIC=ON \
  -DWITH_SIMD=OFF
cmake --build build -j$(nproc)
cmake --install build
cd ..

# Common cross-compilation flags reused by Leptonica, Tesseract, and OpenCV
CMAKE_CROSS="-DCMAKE_SYSTEM_NAME=Windows \
  -DCMAKE_SYSTEM_PROCESSOR=AMD64 \
  -DCMAKE_C_COMPILER=$CC -DCMAKE_CXX_COMPILER=$CXX \
  -DCMAKE_FIND_ROOT_PATH=$DESTDIR \
  -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY \
  -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY \
  -DCMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ONLY \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY"

# ----- Leptonica -----
echo "=== Building Leptonica ==="
curl -f -sL https://github.com/DanBloomberg/leptonica/archive/refs/tags/1.83.1.tar.gz -o leptonica.tar.gz && tar xzf leptonica.tar.gz
cd leptonica-1.83.1
mkdir build && cd build
# CMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY (in CMAKE_CROSS) is needed so
# Tesseract's cmake can process Leptonica's config. But it falsely detects
# open_memstream/fmemopen as available on Windows — override those explicitly.
cmake .. $CMAKE_CROSS \
  -DCMAKE_INSTALL_PREFIX="$DESTDIR" -DBUILD_SHARED_LIBS=OFF \
  -DCMAKE_PREFIX_PATH="$DESTDIR" \
  -DSW_BUILD=OFF \
  -DHAVE_FMEMOPEN=0 -DHAVE_OPEN_MEMSTREAM=0
make -j$(nproc)
make install
# Leptonica may install as libleptonica-X.Y.Z.a (versioned) or liblept.a.
# gosseract links -lleptonica, so ensure libleptonica.a exists unversioned.
cd "$DESTDIR/lib"
for f in libleptonica-*.a libleptonica.a liblept.a; do
  [ -f "$f" ] && { ln -sf "$f" libleptonica.a 2>/dev/null; ln -sf "$f" liblept.a 2>/dev/null; break; }
done
ls -la "$DESTDIR/lib/liblept"*

# Replace the auto-generated Leptonica cmake config with a minimal hand-written
# version. The auto-generated config contains imported targets with cross-compiled
# paths that trigger a std::length_error crash inside cmake when Tesseract tries
# to process them.
LEPT_CMAKE_DIR="$DESTDIR/lib/cmake/leptonica"
rm -rf "$LEPT_CMAKE_DIR"
mkdir -p "$LEPT_CMAKE_DIR"
cat > "$LEPT_CMAKE_DIR/LeptonicaConfig.cmake" << 'LEPTCFG'
# Minimal Leptonica config for cross-compilation (replaces auto-generated one)
set(Leptonica_FOUND TRUE)
set(Leptonica_VERSION "1.83.1")
set(Leptonica_VERSION_MAJOR 1)
set(Leptonica_VERSION_MINOR 83)
set(Leptonica_VERSION_PATCH 1)

get_filename_component(_lept_prefix "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
set(Leptonica_INCLUDE_DIRS "${_lept_prefix}/include" "${_lept_prefix}/include/leptonica")
set(LEPTONICA_INCLUDE_DIRS "${Leptonica_INCLUDE_DIRS}")

# Find the static library
find_library(Leptonica_LIBRARY
  NAMES leptonica leptonica-1.83.1 lept
  PATHS "${_lept_prefix}/lib"
  NO_DEFAULT_PATH)
set(Leptonica_LIBRARIES "${Leptonica_LIBRARY}")
set(LEPTONICA_LIBRARIES "${Leptonica_LIBRARY}")

# Create an imported target if it doesn't exist
if(NOT TARGET leptonica)
  add_library(leptonica STATIC IMPORTED)
  set_target_properties(leptonica PROPERTIES
    IMPORTED_LOCATION "${Leptonica_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${Leptonica_INCLUDE_DIRS}"
  )
endif()

unset(_lept_prefix)
LEPTCFG

cat > "$LEPT_CMAKE_DIR/LeptonicaConfig-version.cmake" << 'LEPTVER'
set(PACKAGE_VERSION "1.83.1")
if("${PACKAGE_FIND_VERSION}" VERSION_GREATER PACKAGE_VERSION)
  set(PACKAGE_VERSION_COMPATIBLE FALSE)
else()
  set(PACKAGE_VERSION_COMPATIBLE TRUE)
  if("${PACKAGE_FIND_VERSION}" VERSION_EQUAL PACKAGE_VERSION)
    set(PACKAGE_VERSION_EXACT TRUE)
  endif()
endif()
LEPTVER

echo "=== Installed minimal LeptonicaConfig.cmake ==="
cat "$LEPT_CMAKE_DIR/LeptonicaConfig.cmake"

cd "$BUILD_DIR"
cd ../..

# ----- Tesseract 5.5.0 -----
echo "=== Building Tesseract ==="
curl -f -sL https://github.com/tesseract-ocr/tesseract/archive/refs/tags/5.5.0.tar.gz -o tesseract.tar.gz && tar xzf tesseract.tar.gz
cd tesseract-5.5.0
mkdir build && cd build
cmake .. $CMAKE_CROSS \
  -DCMAKE_INSTALL_PREFIX="$DESTDIR" -DBUILD_SHARED_LIBS=OFF \
  -DCMAKE_PREFIX_PATH="$DESTDIR" -DLeptonica_DIR="$DESTDIR/lib/cmake/leptonica" \
  -DCMAKE_CXX_STANDARD=17 \
  -DSW_BUILD=OFF \
  -DBUILD_TRAINING_TOOLS=OFF -DBUILD_TESTS=OFF \
  -DDISABLE_CURL=ON -DDISABLE_ARCHIVE=ON \
  -DGRAPHICS_DISABLED=ON
# Build only the library; skip the CLI exe (it needs -lWs2_32 which has
# case-sensitivity issues on Linux and we don't ship the exe anyway).
cmake --build . -j$(nproc) --target libtesseract

# cmake --install would fail on the missing exe, so install manually.
cp libtesseract*.a "$DESTDIR/lib/"
# Also create an unversioned symlink so -ltesseract works
cd "$DESTDIR/lib" && for f in libtesseract[0-9]*.a; do [ -f "$f" ] && ln -sf "$f" libtesseract.a; done && cd -
mkdir -p "$DESTDIR/include/tesseract"
cp ../include/tesseract/*.h "$DESTDIR/include/tesseract/"
# Generated headers (version.h, export.h) live in the build tree
find include -name '*.h' -exec cp {} "$DESTDIR/include/tesseract/" \; 2>/dev/null || true
# pkg-config
mkdir -p "$DESTDIR/lib/pkgconfig"
cp tesseract.pc "$DESTDIR/lib/pkgconfig/" 2>/dev/null || true
cd ../..

# ----- OpenCV 4.13.0 -----
# Build ALL modules (including contrib) so that ALL headers are installed.
# CGO compiles every .cpp file in the gocv package directory regardless of Go
# build tags, so headers for aruco, dnn, calib3d, etc. must exist even though
# we only link core + imgproc + imgcodecs at link time (via customenv).
echo "=== Building OpenCV ==="
curl -f -sL https://github.com/opencv/opencv/archive/refs/tags/4.13.0.tar.gz -o opencv.tar.gz && tar xzf opencv.tar.gz
curl -f -sL https://github.com/opencv/opencv_contrib/archive/refs/tags/4.13.0.tar.gz -o opencv_contrib.tar.gz && tar xzf opencv_contrib.tar.gz
cd opencv-4.13.0
mkdir build && cd build
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
  -DOPENCV_EXTRA_MODULES_PATH=../../opencv_contrib-4.13.0/modules
make -j$(nproc)
make install
cd ../..

# gocv's cgo.go hardcodes versioned lib names for Windows (e.g. -lopencv_core4130).
# OpenCV installs as libopencv_core.a; create 4130 symlinks so the linker finds them.
cd "$DESTDIR/lib"
for f in libopencv_*.a; do
  [ -f "$f" ] || continue
  base="${f%.a}"
  base="${base#lib}"
  [ -e "lib${base}4130.a" ] || ln -sf "$f" "lib${base}4130.a"
done
cd "$BUILD_DIR"

echo "=== Static build complete: $DESTDIR ==="
echo "Libraries:"
ls -la "$DESTDIR/lib/libopencv"* "$DESTDIR/lib/libtesseract"* "$DESTDIR/lib/liblept"* "$DESTDIR/lib/libleptonica"* 2>/dev/null
echo "3rdparty:"
ls -la "$DESTDIR/lib/opencv4/3rdparty/" 2>/dev/null || echo "(none)"
