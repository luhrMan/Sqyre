# shellcheck shell=bash
# Source after setting ABIS: builds Leptonica + Tesseract for each ABI (no-op when present)
# and exports what leptonica-sys / tesseract-sys and the final link need.
_ocr_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_ocr_here/../lib/repo-root.sh"

ANDROID_ABIS="$ABIS" bash "$_ocr_here/build-ocr.sh"
export PKG_CONFIG_ALLOW_CROSS=1
for _abi in $ABIS; do
	case "$_abi" in
	arm64-v8a) _triple=aarch64_linux_android ;;
	x86_64) _triple=x86_64_linux_android ;;
	esac
	export "PKG_CONFIG_LIBDIR_${_triple}=$REPO_ROOT/target/android/ocr/$_abi/lib/pkgconfig"
done
# Both libraries are static: repeat Leptonica after Tesseract and add the NDK's static
# C++ runtime, which pkg-config does not list.
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-lleptonica -C link-arg=-lc++_static -C link-arg=-lc++abi"
unset _ocr_here _abi _triple
