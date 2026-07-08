#!/usr/bin/env bash
# Convert downloaded ONNX vision models to mmap-friendly ORT format.
# Requires python3 and pip. Uses onnxruntime (prefers 1.26.0, falls back to latest on PyPI).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ORT_VERSION="${SQUIRE_ORT_PIP_VERSION:-1.26.0}"
PYDEPS="$ROOT/scripts/vision/.cache/pydeps"

if ! command -v python3 >/dev/null; then
  echo "python3 is required to convert ONNX models to ORT format" >&2
  exit 1
fi
if ! python3 -m pip --version >/dev/null 2>&1; then
  echo "pip is required (install python3-pip) to fetch onnxruntime for ORT conversion" >&2
  exit 1
fi

ensure_python_ort() {
  if python3 -c "import onnxruntime, onnx" 2>/dev/null; then
    return 0
  fi
  if [[ -f "$PYDEPS/onnxruntime/__init__.py" && -f "$PYDEPS/onnx/__init__.py" ]]; then
    return 0
  fi

  echo "Installing onnxruntime (prefer ${ORT_VERSION}) and onnx into $PYDEPS..."
  mkdir -p "$PYDEPS"
  if ! python3 -m pip install --upgrade --target "$PYDEPS" "onnxruntime==${ORT_VERSION}" onnx; then
    echo "onnxruntime==${ORT_VERSION} unavailable; installing latest onnxruntime from PyPI..." >&2
    python3 -m pip install --upgrade --target "$PYDEPS" onnxruntime onnx
  fi
}

python_ort() {
  if python3 -c "import onnxruntime, onnx" 2>/dev/null; then
    python3 "$@"
  else
    PYTHONPATH="$PYDEPS${PYTHONPATH:+:$PYTHONPATH}" python3 "$@"
  fi
}

convert_one() {
  local onnx="$1"
  if [[ ! -f "$onnx" ]]; then
    echo "missing ONNX model: $onnx" >&2
    return 1
  fi

  local dir stem dest
  dir="$(dirname "$onnx")"
  stem="$(basename "$onnx" .onnx)"
  dest="$dir/${stem}.ort"

  if [[ -f "$dest" && "$dest" -nt "$onnx" ]]; then
    echo "Up to date: $dest"
    return 0
  fi

  echo "Converting $(basename "$onnx") -> ${stem}.ort..."
  run_ort_convert "$onnx"

  local produced=""
  for candidate in \
    "$dir/${stem}.ort" \
    "$dir/${stem}.all.ort" \
    "$dir/${stem}.with_runtime_opt.ort"; do
    if [[ -f "$candidate" ]]; then
      produced="$candidate"
      break
    fi
  done
  if [[ -z "$produced" ]]; then
    echo "ORT conversion produced no .ort file for $onnx" >&2
    return 1
  fi
  if [[ "$produced" != "$dest" ]]; then
    mv -f "$produced" "$dest"
  fi
  rm -f "$dir/required_operators.config" "$dir/${stem}.with_runtime_opt.ort"
  echo "Wrote $dest"
}

ort_convert_help() {
  python_ort -m onnxruntime.tools.convert_onnx_models_to_ort --help 2>&1
}

run_ort_convert() {
  local onnx="$1"
  local help platform
  help="$(ort_convert_help)"
  case "$(uname -m)" in
    x86_64|amd64) platform=amd64 ;;
    aarch64|arm64) platform=arm ;;
    *) platform=amd64 ;;
  esac

  if grep -q -- '--optimization_style' <<<"$help"; then
  # onnxruntime >= ~1.24: bake in CPU optimizations (matches session_options DisableAll at load).
    python_ort -m onnxruntime.tools.convert_onnx_models_to_ort \
      --optimization_style Fixed \
      --target_platform "$platform" \
      "$onnx"
  elif grep -q -- '--optimization_level' <<<"$help"; then
    python_ort -m onnxruntime.tools.convert_onnx_models_to_ort \
      --optimization_level all \
      "$onnx"
  else
    echo "unsupported onnxruntime convert_onnx_models_to_ort CLI" >&2
    return 1
  fi
}

ensure_python_ort

if [[ $# -eq 0 ]]; then
  echo "usage: $0 <model.onnx> [...]" >&2
  exit 1
fi

for onnx in "$@"; do
  convert_one "$onnx"
done
