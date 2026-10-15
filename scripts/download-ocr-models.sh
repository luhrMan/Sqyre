#!/usr/bin/env bash
# Download the ocrs detection + recognition models for local OCR (see assets/ocr-models/).
# The files are gitignored; run this after clone or when missing.
#
# The pair is embedded into the binary via include_bytes! from
# crates/sqyre-vision/src/ocr_engine.rs (SQYRE_OCR_MODELS overrides them with a
# directory at runtime). The upstream objects are unversioned — the hashes are
# the pin.
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/lib/repo-root.sh"
MODEL_DIR="$REPO_ROOT/assets/ocr-models"

mkdir -p "$MODEL_DIR"

# Both ONNX file targets in the Makefile run this script; serialize concurrent
# `make -j` invocations so they don't write the same *.partial file.
LOCK_DIR="$MODEL_DIR/.download-lock"
while ! mkdir "$LOCK_DIR" 2>/dev/null; do
	sleep 1
done
trap 'rmdir "$LOCK_DIR"' EXIT

fetch() {
	local name="$1" url="$2" sha256="$3"
	local dest="$MODEL_DIR/$name"
	local partial="$dest.partial"
	if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$sha256" ]; then
		echo "$name already present (sha256 verified)."
		return
	fi
	echo "Downloading $name into $MODEL_DIR ..."
	curl -sSL -o "$partial" "$url"
	local actual
	actual="$(sha256sum "$partial" | cut -d' ' -f1)"
	if [ "$actual" != "$sha256" ]; then
		rm -f "$partial"
		echo "$name SHA-256 mismatch:" >&2
		echo "  expected $sha256" >&2
		echo "  actual   $actual" >&2
		exit 1
	fi
	mv "$partial" "$dest"
	echo "Done ($name, $(du -h "$dest" | cut -f1), sha256 verified)."
}

fetch text-detection.onnx \
	"https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx" \
	"a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe"
fetch text-recognition.onnx \
	"https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx" \
	"86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5"
