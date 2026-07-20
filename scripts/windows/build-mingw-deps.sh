#!/usr/bin/env bash
# Build static zlib/libpng/libjpeg/leptonica/tesseract for MinGW x86_64.
# Adapted from the former Go fyne-cross scripts/windows/docker/build-static-libs.sh
# (OpenCV omitted — Rust uses purecv).
#
# Run inside the Windows cross image build. Installs to DESTDIR.
set -euo pipefail
set -x

DESTDIR="${1:-/usr/local/mingw64-static}"
BUILD_DIR="${BUILD_DIR:-/tmp/staticbuild}"
export PKG_CONFIG_PATH="$DESTDIR/lib/pkgconfig"
export PATH="$DESTDIR/bin:$PATH"

CC=x86_64-w64-mingw32-gcc-posix
CXX=x86_64-w64-mingw32-g++-posix
command -v "$CC" >/dev/null 2>&1 || CC=x86_64-w64-mingw32-gcc
command -v "$CXX" >/dev/null 2>&1 || CXX=x86_64-w64-mingw32-g++
export CC CXX

ZLIB_VER="${ZLIB_VER:-1.3.1}"
LIBPNG_VER="${LIBPNG_VER:-1.6.43}"
LIBJPEG_VER="${LIBJPEG_VER:-2.1.5.1}"
LEPTONICA_VER="${LEPTONICA_VER:-1.83.1}"
TESSERACT_VER="${TESSERACT_VER:-5.5.0}"

mkdir -p "$DESTDIR" "$BUILD_DIR"
cd "$BUILD_DIR"

fetch() {
  local url="$1" out="$2"
  curl -f -sL "$url" -o "$out"
}

echo "=== zlib ${ZLIB_VER} ==="
fetch "https://github.com/madler/zlib/archive/refs/tags/v${ZLIB_VER}.tar.gz" zlib.tar.gz
rm -rf "zlib-${ZLIB_VER}"
tar xzf zlib.tar.gz
(
  cd "zlib-${ZLIB_VER}"
  cmake -B build -G "Unix Makefiles" \
    -DCMAKE_SYSTEM_NAME=Windows -DCMAKE_C_COMPILER="$CC" \
    -DCMAKE_INSTALL_PREFIX="$DESTDIR" \
    -DZLIB_BUILD_SHARED=OFF -DZLIB_BUILD_STATIC=ON
  cmake --build build -j"$(nproc)"
  cmake --install build
  # Dependents look for -lz; cmake may install libzlibstatic.a
  if [ -f "$DESTDIR/lib/libzlibstatic.a" ] && [ ! -f "$DESTDIR/lib/libz.a" ]; then
    ln -sf libzlibstatic.a "$DESTDIR/lib/libz.a"
  fi
)

echo "=== libpng ${LIBPNG_VER} ==="
fetch "https://sourceforge.net/projects/libpng/files/libpng16/${LIBPNG_VER}/libpng-${LIBPNG_VER}.tar.gz/download" \
  libpng.tar.gz
rm -rf "libpng-${LIBPNG_VER}"
tar xzf libpng.tar.gz
(
  cd "libpng-${LIBPNG_VER}"
  export CPPFLAGS="-I$DESTDIR/include"
  export LDFLAGS="-L$DESTDIR/lib"
  ./configure --host=x86_64-w64-mingw32 --prefix="$DESTDIR" \
    --enable-static --disable-shared
  make -j"$(nproc)"
  make install
)
unset CPPFLAGS LDFLAGS

echo "=== libjpeg-turbo ${LIBJPEG_VER} ==="
fetch "https://github.com/libjpeg-turbo/libjpeg-turbo/archive/refs/tags/${LIBJPEG_VER}.tar.gz" \
  jpeg.tar.gz
rm -rf "libjpeg-turbo-${LIBJPEG_VER}"
tar xzf jpeg.tar.gz
(
  cd "libjpeg-turbo-${LIBJPEG_VER}"
  cmake -B build -G "Unix Makefiles" \
    -DCMAKE_SYSTEM_NAME=Windows -DCMAKE_SYSTEM_PROCESSOR=x86_64 \
    -DCMAKE_C_COMPILER="$CC" -DCMAKE_INSTALL_PREFIX="$DESTDIR" \
    -DENABLE_SHARED=OFF -DENABLE_STATIC=ON \
    -DWITH_SIMD=OFF
  cmake --build build -j"$(nproc)"
  cmake --install build
)

# Shared cmake cross flags (from former Go build-static-libs.sh).
# CMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY avoids running Windows test exes.
# shellcheck disable=SC2089
CMAKE_CROSS="-DCMAKE_SYSTEM_NAME=Windows \
  -DCMAKE_SYSTEM_PROCESSOR=AMD64 \
  -DCMAKE_C_COMPILER=$CC -DCMAKE_CXX_COMPILER=$CXX \
  -DCMAKE_FIND_ROOT_PATH=$DESTDIR \
  -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY \
  -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY \
  -DCMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ONLY \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY"

echo "=== leptonica ${LEPTONICA_VER} ==="
fetch "https://github.com/DanBloomberg/leptonica/archive/refs/tags/${LEPTONICA_VER}.tar.gz" \
  leptonica.tar.gz
rm -rf "leptonica-${LEPTONICA_VER}"
tar xzf leptonica.tar.gz
(
  cd "leptonica-${LEPTONICA_VER}"
  mkdir build && cd build
  # Override false-positive open_memstream/fmemopen detection under STATIC_LIBRARY try_compile.
  # shellcheck disable=SC2090
  cmake .. $CMAKE_CROSS \
    -DCMAKE_INSTALL_PREFIX="$DESTDIR" -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_PREFIX_PATH="$DESTDIR" \
    -DSW_BUILD=OFF \
    -DHAVE_FMEMOPEN=0 -DHAVE_OPEN_MEMSTREAM=0
  make -j"$(nproc)"
  make install
)
(
  cd "$DESTDIR/lib"
  for f in libleptonica-*.a libleptonica.a liblept.a; do
    if [ -f "$f" ]; then
      ln -sf "$f" libleptonica.a
      ln -sf "$f" liblept.a
      break
    fi
  done
  ls -la "$DESTDIR/lib/liblept"*
)

# Minimal LeptonicaConfig.cmake — auto-generated configs crash cmake when
# Tesseract consumes them during MinGW cross (same fix as Go pipeline).
LEPT_CMAKE_DIR="$DESTDIR/lib/cmake/leptonica"
rm -rf "$LEPT_CMAKE_DIR"
mkdir -p "$LEPT_CMAKE_DIR"
cat > "$LEPT_CMAKE_DIR/LeptonicaConfig.cmake" << 'LEPTCFG'
set(Leptonica_FOUND TRUE)
set(Leptonica_VERSION "1.83.1")
set(Leptonica_VERSION_MAJOR 1)
set(Leptonica_VERSION_MINOR 83)
set(Leptonica_VERSION_PATCH 1)
get_filename_component(_lept_prefix "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
set(Leptonica_INCLUDE_DIRS "${_lept_prefix}/include" "${_lept_prefix}/include/leptonica")
set(LEPTONICA_INCLUDE_DIRS "${Leptonica_INCLUDE_DIRS}")
find_library(Leptonica_LIBRARY
  NAMES leptonica leptonica-1.83.1 lept
  PATHS "${_lept_prefix}/lib"
  NO_DEFAULT_PATH)
set(Leptonica_LIBRARIES "${Leptonica_LIBRARY}")
set(LEPTONICA_LIBRARIES "${Leptonica_LIBRARY}")
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

# pkg-config for leptess (leptonica-sys probes "lept")
mkdir -p "$DESTDIR/lib/pkgconfig"
if [ ! -f "$DESTDIR/lib/pkgconfig/lept.pc" ]; then
  cat > "$DESTDIR/lib/pkgconfig/lept.pc" << EOF
prefix=$DESTDIR
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: lept
Description: Leptonica image processing library
Version: ${LEPTONICA_VER}
Libs: -L\${libdir} -lleptonica
Libs.private: -lpng16 -ljpeg -lz
Cflags: -I\${includedir}/leptonica
EOF
fi

echo "=== tesseract ${TESSERACT_VER} ==="
fetch "https://github.com/tesseract-ocr/tesseract/archive/refs/tags/${TESSERACT_VER}.tar.gz" \
  tesseract.tar.gz
rm -rf "tesseract-${TESSERACT_VER}"
tar xzf tesseract.tar.gz
(
  cd "tesseract-${TESSERACT_VER}"
  mkdir build && cd build
  # shellcheck disable=SC2090
  cmake .. $CMAKE_CROSS \
    -DCMAKE_INSTALL_PREFIX="$DESTDIR" -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_PREFIX_PATH="$DESTDIR" \
    -DLeptonica_DIR="$DESTDIR/lib/cmake/leptonica" \
    -DCMAKE_CXX_STANDARD=17 \
    -DSW_BUILD=OFF \
    -DBUILD_TRAINING_TOOLS=OFF -DBUILD_TESTS=OFF \
    -DDISABLE_CURL=ON -DDISABLE_ARCHIVE=ON \
    -DGRAPHICS_DISABLED=ON
  # Library only — skip CLI (Ws2_32 / optional image deps).
  cmake --build . -j"$(nproc)" --target libtesseract
  cp libtesseract*.a "$DESTDIR/lib/"
  (
    cd "$DESTDIR/lib"
    for f in libtesseract[0-9]*.a libtesseract.a; do
      if [ -f "$f" ]; then
        ln -sf "$f" libtesseract.a
        break
      fi
    done
  )
  mkdir -p "$DESTDIR/include/tesseract"
  cp ../include/tesseract/*.h "$DESTDIR/include/tesseract/"
  find include -name '*.h' -exec cp {} "$DESTDIR/include/tesseract/" \; 2>/dev/null || true
  mkdir -p "$DESTDIR/lib/pkgconfig"
  if [ -f tesseract.pc ]; then
    cp tesseract.pc "$DESTDIR/lib/pkgconfig/"
  else
    cat > "$DESTDIR/lib/pkgconfig/tesseract.pc" << EOF
prefix=$DESTDIR
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: tesseract
Description: Tesseract OCR
Version: ${TESSERACT_VER}
Requires: lept
Libs: -L\${libdir} -ltesseract
Libs.private: -lstdc++
Cflags: -I\${includedir}
EOF
  fi
)

# Ensure x86_64-w64-mingw32-pkg-config can see our .pc files when PREFIX differs.
"${CC%-gcc*}"-pkg-config --exists tesseract || true
"${CC%-gcc*}"-pkg-config --exists lept || PKG_CONFIG_PATH="$DESTDIR/lib/pkgconfig" \
  pkg-config --exists lept
PKG_CONFIG_PATH="$DESTDIR/lib/pkgconfig" pkg-config --modversion tesseract
PKG_CONFIG_PATH="$DESTDIR/lib/pkgconfig" pkg-config --libs --static tesseract

echo "=== Static MinGW OCR deps installed under ${DESTDIR} ==="
ls -la "$DESTDIR/lib/libtesseract"* "$DESTDIR/lib/liblept"* "$DESTDIR/lib/libpng"* "$DESTDIR/lib/libz"* "$DESTDIR/lib/libjpeg"* 2>/dev/null || true
rm -rf "$BUILD_DIR"
