#!/usr/bin/env bash
# Build the sideloadable Android APK: Rust cdylib via cargo-ndk, then the Gradle shell.
#
# Needs ANDROID_HOME, ANDROID_NDK_HOME, cargo-ndk, gradle, JDK 17 (all in the
# devcontainer). The ocrs ONNX models are embedded into libsqyre_app.so via
# include_bytes! (assets/ocr-models/, see scripts/download-ocr-models.sh).
#
# Env:
#   ANDROID_FEATURES  sqyre-app features (default: native-runtime,overlay-buttons;
#                     empty = editor only, the same surface as `make wasm`)
#   ANDROID_PROFILE   debug | release (default: debug)
#   ANDROID_ABIS      space-separated ABIs: arm64-v8a, x86_64 (default: both; x86_64 is
#                     for the emulator)
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"

FEATURES="${ANDROID_FEATURES-native-runtime,overlay-buttons}"
PROFILE="${ANDROID_PROFILE:-debug}"
ABIS="${ANDROID_ABIS:-arm64-v8a x86_64}"
MIN_SDK=29

for tool in cargo-ndk gradle; do
	command -v "$tool" >/dev/null 2>&1 || {
		echo "$tool not found (open the devcontainer, or see docs/ANDROID.md)" >&2
		exit 1
	}
done
: "${ANDROID_HOME:?ANDROID_HOME is not set}"
: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME is not set}"

cargo_args=(build -p sqyre-app --lib --no-default-features)
if [ -n "$FEATURES" ]; then
	cargo_args+=(--features "$FEATURES")
fi
if [ "$PROFILE" = release ]; then
	cargo_args+=(--release)
fi

ndk_targets=()
for abi in $ABIS; do
	ndk_targets+=(-t "$abi")
done

# OCR models are embedded at compile time — required for cargo ndk.
model_dir="$REPO_ROOT/assets/ocr-models"
[ -f "$model_dir/text-detection.onnx" ] && [ -f "$model_dir/text-recognition.onnx" ] ||
	bash "$REPO_ROOT/scripts/download-ocr-models.sh"

ASSETS="$REPO_ROOT/android/app/src/main/assets"
rm -rf "$ASSETS"

JNI_LIBS="$REPO_ROOT/android/app/src/main/jniLibs"
rm -rf "$JNI_LIBS"
(cd "$REPO_ROOT" && cargo ndk "${ndk_targets[@]}" --platform "$MIN_SDK" -o "$JNI_LIBS" "${cargo_args[@]}")

if [ "$PROFILE" = release ]; then
	gradle_task=assembleRelease
else
	gradle_task=assembleDebug
fi
(cd "$REPO_ROOT/android" && gradle --no-daemon "$gradle_task")

mkdir -p "$REPO_ROOT/bin"
apk="$(find "$REPO_ROOT/android/app/build/outputs/apk/$PROFILE" -name '*.apk' | head -n 1)"
[ -n "$apk" ] || {
	echo "no APK produced under android/app/build/outputs/apk/$PROFILE" >&2
	exit 1
}
cp "$apk" "$REPO_ROOT/bin/sqyre-$PROFILE.apk"
echo "APK: $REPO_ROOT/bin/sqyre-$PROFILE.apk"
