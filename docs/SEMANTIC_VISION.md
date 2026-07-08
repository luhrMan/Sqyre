# Semantic Vision Detection

Open-vocabulary object detection for **Semantic Search** macros — describe what to find in plain text (e.g. *"All Healing potions"*, *"Metal Armor"*) without pre-captured icon templates.

## Architecture: lean Sqyre + optional `sqyre-vision` worker

| Binary | Size | Role |
|--------|------|------|
| **`sqyre`** | Lean (default) | UI, macros, `RemoteDetector` → shells out to worker |
| **`sqyre-vision`** | Large (~300MB+ with embed) | YOLO-World + CLIP ONNX inference only |

```
sqyre  ──JSON/stdin──►  sqyre-vision detect  ──►  bounding boxes
         temp PNG            ONNX models
```

The main app never links ONNX models. Users choose:

1. **Vision AppImage / embedded worker** — `sqyre-vision` beside `sqyre`, models embedded in worker binary (`vision_embed` build).
2. **Lean install + external models** — build `sqyre-vision` without embed, run `scripts/vision/download-models.sh`, point Settings → models directory at `~/.sqyre/models`.
3. **Custom worker path** — Settings → worker binary path for a self-built `sqyre-vision`.

## Settings (Sqyre → Settings → Semantic vision)

| Preference | Purpose |
|------------|---------|
| Worker binary path | Path to `sqyre-vision`; empty = auto-detect sibling of `sqyre` or `PATH` |
| Models directory | Passed to worker as `SQUIRE_VISION_MODEL_DIR`; default `~/.sqyre/models` |

Environment variables (AppImage / advanced):

| Variable | Purpose |
|----------|---------|
| `SQUIRE_VISION_WORKER` | Worker binary path (overrides auto-detect) |
| `SQUIRE_VISION_MODEL_DIR` | ONNX model directory |
| `SQUIRE_ORT_LIB` | Path to `libonnxruntime.so` |

## Build

```bash
# Lean main app (default)
make linux

# Embedded vision worker (downloads models, embeds in binary)
make sqyre-vision-embed

# bin/sqyre + embedded bin/sqyre-vision
make linux-vision

# Lean worker (uses ~/.sqyre/models; run make vision-models first)
make sqyre-vision

# Vision AppImage
make appimage-vision
```

Place `sqyre-vision` next to `sqyre`, or set the worker path in Settings.

## AppImage

| Script | Output |
|--------|--------|
| `scripts/linux/packaging/appimage/build-appimage.sh` | Lean Sqyre only |
| `scripts/linux/packaging/appimage/build-appimage-vision.sh` | Sqyre + `sqyre-vision` + ORT; sets `SQUIRE_VISION_WORKER` |

## Models (`~/.sqyre/models/`)

| File | Role |
|------|------|
| `yolov8s-worldv2.onnx` | Source model from [Instemic/yolo-world-onnx](https://huggingface.co/Instemic/yolo-world-onnx) |
| `clip-text-vit-b32.onnx` | Source model from [inference4j/clip-vit-base-patch32](https://huggingface.co/inference4j/clip-vit-base-patch32) |

Download via `make vision-models`. Embedded vision builds ship source `.onnx` only; on first semantic-vision load they are extracted to `~/.sqyre/models/` and optimized caches are written alongside:

- `*.optimized.onnx` — created in-process via ONNX Runtime (default, no Python required)
- `*.ort` — optional mmap-friendly cache if `SQUIRE_VISION_CONVERT_SCRIPT` points at `scripts/vision/convert-models-to-ort.sh` and Python onnxruntime is installed

CLIP BPE tokenizer (`vocab.json`, `merges.txt`, ~1.4 MB) is embedded in `sqyre-vision` like Tesseract `eng.traineddata` — gitignored, fetched with `make clip-tokenizer`.

## Worker protocol

```bash
sqyre-vision detect < request.json
```

Request (stdin JSON):

```json
{
  "prompts": ["healing potions"],
  "image_path": "/tmp/frame.png",
  "confidence_threshold": 0.25,
  "iou_threshold": 0.45,
  "max_matches": 10
}
```

Response (stdout JSON):

```json
{
  "detections": [
    {"label": "healing potions", "confidence": 0.87, "bounds": {"min_x": 10, "min_y": 20, "max_x": 50, "max_y": 60}}
  ]
}
```

Subcommands: `ping`, `detect`, `info`.

## Macro YAML

```yaml
- type: semanticsearch
  name: find potions
  prompt: "All Healing potions"
  searcharea:
    program: MyGame
    name: Inventory
  confidencethreshold: 0.3
  maxmatches: 10
  outputxvariable: foundX
  outputyvariable: foundY
  outputlabelvariable: foundLabel
  subactions:
    - type: click
      button: left
```

## Testing

```bash
./scripts/test.sh ./internal/vision/...
```

Integration with real models:

```bash
./scripts/vision/download-models.sh
export SQUIRE_ORT_LIB=scripts/vision/.cache/libonnxruntime.so
export SQUIRE_VISION_MODEL_DIR=~/.sqyre/models
./bin/sqyre-vision info
```
