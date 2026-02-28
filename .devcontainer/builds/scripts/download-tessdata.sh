#!/usr/bin/env bash
# Download eng.traineddata for embedding (see internal/assets/tessdata.go).
# The file is gitignored; run this after clone or when missing.
set -e
# Script lives in .devcontainer/builds/scripts/; go up to repo root.
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TESSDATA_DIR="$REPO_ROOT/internal/assets/tessdata"
URL="https://github.com/tesseract-ocr/tessdata/raw/main/eng.traineddata"
mkdir -p "$TESSDATA_DIR"
echo "Downloading eng.traineddata into $TESSDATA_DIR ..."
curl -sSL -o "$TESSDATA_DIR/eng.traineddata" "$URL"
echo "Done ($(du -h "$TESSDATA_DIR/eng.traineddata" | cut -f1))."
