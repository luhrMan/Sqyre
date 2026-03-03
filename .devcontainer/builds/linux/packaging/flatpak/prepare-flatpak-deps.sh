#!/usr/bin/env bash
# Prepare flatpak-deps/ from host OpenCV, Tesseract, and Leptonica.
# Run this inside the devcontainer (or any host that has OpenCV in /usr/local
# and Tesseract/Leptonica from apt). Then build with:
#   flatpak-builder --user --force-clean build-dir com.sqyre.app.with-host-deps.yml
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPS_DIR="$SCRIPT_DIR/flatpak-deps"
SYS_LIB="${SYS_LIB:-/usr/lib/x86_64-linux-gnu}"

rm -rf "$DEPS_DIR"
mkdir -p "$DEPS_DIR"/{include,lib/pkgconfig}

# OpenCV (devcontainer builds to /usr/local)
OPENCV_PC=""
OPENCV_LIB=""
[ -f /usr/local/lib/pkgconfig/opencv4.pc ] && OPENCV_PC=/usr/local/lib/pkgconfig/opencv4.pc && OPENCV_LIB=/usr/local/lib
[ -z "$OPENCV_PC" ] && [ -f /usr/local/lib64/pkgconfig/opencv4.pc ] && OPENCV_PC=/usr/local/lib64/pkgconfig/opencv4.pc && OPENCV_LIB=/usr/local/lib64
if [ -n "$OPENCV_PC" ] && [ -d /usr/local/include/opencv4 ]; then
  cp -a /usr/local/include/opencv4 "$DEPS_DIR/include/"
  cp -a "$OPENCV_LIB"/libopencv_*.so* "$DEPS_DIR/lib/" 2>/dev/null || true
  sed 's|^prefix=.*|prefix=/app|' "$OPENCV_PC" > "$DEPS_DIR/lib/pkgconfig/opencv4.pc"
  sed -i 's|^libdir=.*|libdir=${prefix}/lib|' "$DEPS_DIR/lib/pkgconfig/opencv4.pc"
  sed -i 's|^includedir=.*|includedir=${prefix}/include|' "$DEPS_DIR/lib/pkgconfig/opencv4.pc"
  echo "  OpenCV: copied from /usr/local"
else
  echo "ERROR: OpenCV not found. Need /usr/local/include/opencv4 and /usr/local/lib/pkgconfig/opencv4.pc" >&2
  echo "Run this script inside the devcontainer (which builds OpenCV), or use com.sqyre.app.yml to build from source." >&2
  exit 1
fi

# Tesseract (from apt)
if [ -d /usr/include/tesseract ] && [ -f "$SYS_LIB/pkgconfig/tesseract.pc" ]; then
  cp -a /usr/include/tesseract "$DEPS_DIR/include/"
  cp -a "$SYS_LIB"/libtesseract.so* "$DEPS_DIR/lib/" 2>/dev/null || true
  sed 's|^prefix=.*|prefix=/app|' "$SYS_LIB/pkgconfig/tesseract.pc" > "$DEPS_DIR/lib/pkgconfig/tesseract.pc"
  echo "  Tesseract: copied from $SYS_LIB /usr/include"
else
  echo "WARNING: Tesseract not found (install libtesseract-dev)" >&2
fi

# Leptonica (from apt)
if [ -d /usr/include/leptonica ] && [ -f "$SYS_LIB/pkgconfig/lept.pc" ]; then
  cp -a /usr/include/leptonica "$DEPS_DIR/include/"
  cp -a "$SYS_LIB"/liblept.so* "$SYS_LIB"/libleptonica.so* "$DEPS_DIR/lib/" 2>/dev/null || true
  sed 's|^prefix=.*|prefix=/app|' "$SYS_LIB/pkgconfig/lept.pc" > "$DEPS_DIR/lib/pkgconfig/lept.pc"
  echo "  Leptonica: copied from $SYS_LIB /usr/include"
else
  echo "WARNING: Leptonica not found (install libleptonica-dev)" >&2
fi

# Fix .pc libdir to /app/lib (in case they had libdir=...)
for pc in "$DEPS_DIR/lib/pkgconfig"/*.pc; do
  [ -f "$pc" ] || continue
  sed -i 's|^libdir=.*|libdir=${prefix}/lib|' "$pc"
  sed -i 's|^includedir=.*|includedir=${prefix}/include|' "$pc"
done

echo "flatpak-deps ready at $DEPS_DIR"
ls -la "$DEPS_DIR/lib/" 2>/dev/null | head -20
