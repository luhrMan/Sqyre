#!/usr/bin/env bash
# `make android-check`: cargo check sqyre-app for Android as the editor-only and the
# runtime (default APK) build, plus the platform crates on their own.
#
# Env: ANDROID_ABIS (default: arm64-v8a), CARGO_FLAGS (extra cargo args).
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"

ABIS="${ANDROID_ABIS:-arm64-v8a}"
MIN_SDK=29
read -r -a extra <<<"${CARGO_FLAGS:-}"

ndk_targets=()
for abi in $ABIS; do
	ndk_targets+=(-t "$abi")
done
ndk=(cargo ndk "${ndk_targets[@]}" --platform "$MIN_SDK" check)

cd "$REPO_ROOT"
# OCR models are embedded via include_bytes! — required at compile time.
[ -f assets/ocr-models/text-detection.onnx ] && [ -f assets/ocr-models/text-recognition.onnx ] ||
	bash scripts/download-ocr-models.sh
"${ndk[@]}" -p sqyre-app --lib --no-default-features "${extra[@]}"
"${ndk[@]}" -p sqyre-app --lib --no-default-features --features native-runtime,overlay-buttons "${extra[@]}"
"${ndk[@]}" -p sqyre-capture -p sqyre-input "${extra[@]}"
