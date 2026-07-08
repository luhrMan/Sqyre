#!/usr/bin/env bash
# Download ONNX models and prepare embedded bundle for sqyre-vision.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUNDLED="$ROOT/internal/vision/embedmodels/bundled"
MODELS_DIR="${SQUIRE_VISION_MODEL_DIR:-$HOME/.sqyre/models}"
mkdir -p "$BUNDLED" "$MODELS_DIR"

# Never ship runtime caches in the embed bundle (they bloat the binary).
rm -f "$BUNDLED"/*.ort "$BUNDLED"/*.optimized.onnx

"$ROOT/scripts/vision/download-clip-tokenizer.sh"

YOLO_URL="https://huggingface.co/Instemic/yolo-world-onnx/resolve/main/yolov8s-worldv2.onnx"
CLIP_URL="https://huggingface.co/inference4j/clip-vit-base-patch32/resolve/main/text_model.onnx"

download() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then
    echo "Present: $dest"
    return
  fi
  echo "Downloading $(basename "$dest")..."
  curl -fL "$url" -o "$dest"
}

download "$YOLO_URL" "$MODELS_DIR/yolov8s-worldv2.onnx"
download "$CLIP_URL" "$MODELS_DIR/clip-text-vit-b32.onnx"

cp -f "$MODELS_DIR/yolov8s-worldv2.onnx" "$BUNDLED/yolov8s-worldv2.onnx"
cp -f "$MODELS_DIR/clip-text-vit-b32.onnx" "$BUNDLED/clip-text-vit-b32.onnx"

# Must match github.com/yalue/onnxruntime_go (see go.mod); v1.31.0 uses ORT 1.26.0 C API.
ORT_VERSION="1.26.0"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ORT_ARCH="x64" ;;
  aarch64|arm64) ORT_ARCH="aarch64" ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

ORT_TGZ="onnxruntime-linux-${ORT_ARCH}-${ORT_VERSION}.tgz"
ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_TGZ}"
ORT_CACHE="$ROOT/scripts/vision/.cache"
mkdir -p "$ORT_CACHE"

if [[ ! -f "$ORT_CACHE/$ORT_TGZ" ]]; then
  echo "Downloading ONNX Runtime ${ORT_VERSION}..."
  curl -fL "$ORT_URL" -o "$ORT_CACHE/$ORT_TGZ"
fi

ORT_EXTRACT="$ORT_CACHE/onnxruntime-linux-${ORT_ARCH}-${ORT_VERSION}"
if [[ ! -d "$ORT_EXTRACT" ]]; then
  tar -xzf "$ORT_CACHE/$ORT_TGZ" -C "$ORT_CACHE"
fi

ORT_LIB=""
for candidate in \
  "$ORT_EXTRACT/lib/libonnxruntime.so.${ORT_VERSION}" \
  "$ORT_EXTRACT/lib64/libonnxruntime.so.${ORT_VERSION}"; do
  if [[ -f "$candidate" ]]; then
    ORT_LIB="$candidate"
    break
  fi
done
if [[ -z "$ORT_LIB" ]]; then
  ORT_LIB="$(find "$ORT_EXTRACT" -maxdepth 2 -name 'libonnxruntime.so.*' -type f 2>/dev/null | head -1)"
fi
if [[ -n "$ORT_LIB" && -f "$ORT_LIB" ]]; then
  SQYRE_LIB_DIR="${HOME}/.sqyre/lib"
  mkdir -p "$SQYRE_LIB_DIR" "$BUNDLED"
  cp -f "$ORT_LIB" "$BUNDLED/libonnxruntime.so"
  cp -f "$ORT_LIB" "$ORT_CACHE/libonnxruntime.so"
  cp -f "$ORT_LIB" "$SQYRE_LIB_DIR/libonnxruntime.so"
  echo "Bundled ORT: $BUNDLED/libonnxruntime.so"
  echo "Runtime ORT: $SQYRE_LIB_DIR/libonnxruntime.so"
else
  echo "WARNING: libonnxruntime not found under $ORT_EXTRACT" >&2
fi

echo ""
echo "ONNX models in $MODELS_DIR (optimized caches written there on first vision load)"
echo "Bundled for vision_embed: ONNX + libonnxruntime.so only ($BUNDLED)"
