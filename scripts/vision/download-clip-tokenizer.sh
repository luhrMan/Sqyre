#!/usr/bin/env bash
# Download CLIP ViT-B/32 BPE tokenizer files for embedding (see internal/vision/clipdata/tokenizer.go).
# Files are gitignored; run after clone or when missing.
set -euo pipefail
_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$_here/../lib/repo-root.sh"

CLIPDATA_DIR="$REPO_ROOT/internal/vision/clipdata"
mkdir -p "$CLIPDATA_DIR"

VOCAB_URL="https://huggingface.co/openai/clip-vit-base-patch32/resolve/main/vocab.json"
MERGES_URL="https://huggingface.co/openai/clip-vit-base-patch32/resolve/main/merges.txt"

download() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then
    echo "Present: $dest"
    return
  fi
  echo "Downloading $(basename "$dest")..."
  curl -fsSL -o "$dest" "$url"
  echo "  -> $(du -h "$dest" | cut -f1)"
}

download "$VOCAB_URL" "$CLIPDATA_DIR/vocab.json"
download "$MERGES_URL" "$CLIPDATA_DIR/merges.txt"
