#!/usr/bin/env bash
# Cross-build static Leptonica + Tesseract for Android so leptess links into libsqyre_app.so.
#
# Sqyre hands Tesseract raw pixels, so Leptonica is built without any image codec.
# Output: target/android/ocr/<abi>/{include,lib,lib/pkgconfig}; build-apk.sh points
# pkg-config there. Re-running skips ABIs whose libtesseract.a already exists.
#
# Needs ANDROID_NDK_HOME, cmake, ninja-build or make, curl (all in the devcontainer).
#
# Env:
#   ANDROID_ABIS  space-separated ABIs (default: arm64-v8a)
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"

ABIS="${ANDROID_ABIS:-arm64-v8a}"
MIN_SDK=29

LEPT_VERSION=1.85.0
LEPT_URL="https://github.com/DanBloomberg/leptonica/releases/download/${LEPT_VERSION}/leptonica-${LEPT_VERSION}.tar.gz"
LEPT_SHA256=3745ae3bf271a6801a2292eead83ac926e3a9bc1bf622e9cd4dd0f3786e17205
TESS_VERSION=5.5.1
TESS_URL="https://github.com/tesseract-ocr/tesseract/archive/refs/tags/${TESS_VERSION}.tar.gz"
TESS_SHA256=a7a3f2a7420cb6a6a94d80c24163e183cf1d2f1bed2df3bbc397c81808a57237

: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME is not set}"
command -v cmake >/dev/null 2>&1 || {
	echo "cmake not found (open the devcontainer, or see docs/ANDROID.md)" >&2
	exit 1
}
TOOLCHAIN="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"
[ -f "$TOOLCHAIN" ] || {
	echo "NDK CMake toolchain missing: $TOOLCHAIN" >&2
	exit 1
}

OCR_ROOT="$REPO_ROOT/target/android/ocr"
SRC="$OCR_ROOT/src"
mkdir -p "$SRC"

fetch() {
	local url="$1" sha="$2" dest="$3"
	if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$sha" ]; then
		return
	fi
	curl -fsSL -o "$dest.partial" "$url"
	local actual
	actual="$(sha256sum "$dest.partial" | cut -d' ' -f1)"
	if [ "$actual" != "$sha" ]; then
		rm -f "$dest.partial"
		echo "$(basename "$dest") SHA-256 mismatch: expected $sha, got $actual" >&2
		exit 1
	fi
	mv "$dest.partial" "$dest"
}

unpack() {
	local tarball="$1" dir="$2"
	if [ ! -d "$dir" ]; then
		mkdir -p "$dir.partial"
		tar -xzf "$tarball" -C "$dir.partial" --strip-components=1
		mv "$dir.partial" "$dir"
	fi
}

fetch "$LEPT_URL" "$LEPT_SHA256" "$SRC/leptonica-$LEPT_VERSION.tar.gz"
fetch "$TESS_URL" "$TESS_SHA256" "$SRC/tesseract-$TESS_VERSION.tar.gz"
unpack "$SRC/leptonica-$LEPT_VERSION.tar.gz" "$SRC/leptonica-$LEPT_VERSION"
unpack "$SRC/tesseract-$TESS_VERSION.tar.gz" "$SRC/tesseract-$TESS_VERSION"

generator=()
if command -v ninja >/dev/null 2>&1; then
	generator=(-G Ninja)
fi

for abi in $ABIS; do
	case "$abi" in
	arm64-v8a | x86_64) ;;
	*)
		echo "unsupported ABI for the OCR build: $abi (arm64-v8a, x86_64)" >&2
		exit 1
		;;
	esac
	prefix="$OCR_ROOT/$abi"
	if [ -f "$prefix/lib/libtesseract.a" ] && [ -f "$prefix/lib/libleptonica.a" ]; then
		echo "OCR libs for $abi already built ($prefix)"
		continue
	fi
	common=(
		"${generator[@]}"
		-DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN"
		-DANDROID_ABI="$abi"
		-DANDROID_PLATFORM="android-$MIN_SDK"
		-DANDROID_STL=c++_static
		-DCMAKE_BUILD_TYPE=Release
		-DCMAKE_INSTALL_PREFIX="$prefix"
		-DCMAKE_FIND_ROOT_PATH="$prefix"
		-DCMAKE_POSITION_INDEPENDENT_CODE=ON
		-DBUILD_SHARED_LIBS=OFF
		-DSW_BUILD=OFF
	)

	build="$OCR_ROOT/build/$abi/leptonica"
	rm -rf "$build"
	cmake -S "$SRC/leptonica-$LEPT_VERSION" -B "$build" "${common[@]}" \
		-DBUILD_PROG=OFF \
		-DENABLE_ZLIB=OFF -DENABLE_PNG=OFF -DENABLE_GIF=OFF -DENABLE_JPEG=OFF \
		-DENABLE_TIFF=OFF -DENABLE_WEBP=OFF -DENABLE_OPENJPEG=OFF
	cmake --build "$build" --parallel
	cmake --install "$build"

	# Tesseract's CMake requires the NDK cpu-features compat package on Android, but only
	# 32-bit ARM uses it (arm64 always has NEON, x86_64 uses cpuid). Satisfy the lookup
	# with an empty target instead of patching upstream sources.
	cpu_features="$OCR_ROOT/build/$abi/cpu-features-stub"
	mkdir -p "$cpu_features"
	cat >"$cpu_features/CpuFeaturesNdkCompatConfig.cmake" <<'EOF'
if(NOT TARGET CpuFeatures::ndk_compat)
  add_library(CpuFeatures::ndk_compat INTERFACE IMPORTED)
endif()
EOF

	build="$OCR_ROOT/build/$abi/tesseract"
	rm -rf "$build"
	# LEPT_TIFF_RESULT answers a try_run that cannot execute when cross-compiling:
	# non-zero means Leptonica has no TIFF writer, which is true here.
	cmake -S "$SRC/tesseract-$TESS_VERSION" -B "$build" "${common[@]}" \
		-DLeptonica_DIR="$prefix/lib/cmake/leptonica" \
		-DCpuFeaturesNdkCompat_DIR="$cpu_features" \
		-DLEPT_TIFF_RESULT=1 -DLEPT_TIFF_RESULT__TRYRUN_OUTPUT= \
		-DBUILD_TRAINING_TOOLS=OFF -DBUILD_TESTS=OFF \
		-DDISABLE_ARCHIVE=ON -DDISABLE_CURL=ON -DDISABLE_TIFF=ON \
		-DGRAPHICS_DISABLED=ON -DOPENMP_BUILD=OFF -DENABLE_LTO=OFF \
		-DINSTALL_CONFIGS=OFF
	cmake --build "$build" --parallel
	cmake --install "$build"

	# leptonica-sys probes `lept`; Leptonica's CMake names the file after the build type.
	# Relative prefixes keep the tree usable from any checkout path.
	pc="$prefix/lib/pkgconfig"
	mv "$pc/lept_Release.pc" "$pc/lept.pc"
	sed -i "s|$prefix|\${pcfiledir}/../..|g" "$pc/lept.pc" "$pc/tesseract.pc"
done

echo "OCR libs: $OCR_ROOT/<abi>"
