#!/usr/bin/env bash
# Download eng.traineddata — used only by the Go comparison harness
# (scripts/bench-compare/run-go.sh); Sqyre itself OCRs via ocrs ONNX models
# (scripts/download-ocr-models.sh). The file is gitignored.
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/lib/repo-root.sh"
TESSDATA_DIR="$REPO_ROOT/assets/tessdata"
URL="https://github.com/tesseract-ocr/tessdata/raw/4.1.0/eng.traineddata"
SHA256="daa0c97d651c19fba3b25e81317cd697e9908c8208090c94c3905381c23fc047"

mkdir -p "$TESSDATA_DIR"
DEST="$TESSDATA_DIR/eng.traineddata"
PARTIAL="$DEST.partial"

echo "Downloading eng.traineddata into $TESSDATA_DIR ..."
curl -sSL -o "$PARTIAL" "$URL"

ACTUAL="$(sha256sum "$PARTIAL" | cut -d' ' -f1)"
if [ "$ACTUAL" != "$SHA256" ]; then
	rm -f "$PARTIAL"
	echo "eng.traineddata SHA-256 mismatch:" >&2
	echo "  expected $SHA256" >&2
	echo "  actual   $ACTUAL" >&2
	exit 1
fi

mv "$PARTIAL" "$DEST"
echo "Done ($(du -h "$DEST" | cut -f1), sha256 verified)."
