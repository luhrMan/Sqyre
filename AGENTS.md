# Sqyre - Development Agent Instructions

## Cursor Cloud specific instructions

### Overview

Sqyre is a Go desktop macro builder application using Fyne (GUI), GoCV/OpenCV (computer vision), Gosseract/Tesseract (OCR), and Robotgo (automation). It compiles to a single binary — there are no microservices, databases, or docker-compose stacks.

### Building and running

All building happens inside the devcontainer Docker image (`sqyre-dev`). The image is pre-built and contains Go 1.24, OpenCV 4.6.0, Tesseract, and all X11/OpenGL libraries.

**Build the binary:**
```bash
sudo docker run --rm -v /workspace:/workspace -w /workspace \
  -e GOFLAGS="-tags=gocv_specific_modules" -e CGO_ENABLED=1 \
  sqyre-dev go build -buildvcs=false -o sqyre ./cmd/sqyre
```

**Run the application** (requires Xvfb for headless environments):
```bash
# Start Xvfb if not already running
Xvfb :99 -screen 0 1920x1080x24 &>/dev/null &

# Run via container with X forwarding
sudo docker run --rm -d --name sqyre-app \
  -v /workspace:/workspace \
  -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
  -e DISPLAY=:99 -w /workspace \
  sqyre-dev ./sqyre
```

### Linting

```bash
sudo docker run --rm -v /workspace:/workspace -w /workspace \
  -e GOFLAGS="-tags=gocv_specific_modules" -e CGO_ENABLED=1 \
  sqyre-dev go vet ./...
```

### Testing

Tests must run inside the devcontainer with Xvfb for X11 support. A test fixture file `testdata/db.yaml` is required but not committed — copy `testdata/config.yaml` to `testdata/db.yaml` before running repository tests:

```bash
cp internal/models/repositories/testdata/config.yaml internal/models/repositories/testdata/db.yaml

sudo docker run --rm -v /workspace:/workspace -w /workspace \
  -e GOFLAGS="-tags=gocv_specific_modules" -e CGO_ENABLED=1 -e DISPLAY=:99 \
  sqyre-dev bash -c "apt-get update -qq && apt-get install -y -qq xvfb > /dev/null 2>&1 && \
  Xvfb :99 -screen 0 1920x1080x24 &>/dev/null & sleep 1 && go test -v -count=1 ./..."
```

### Known issues (pre-existing)

- `TestGetVariants/Legacy_icon_without_variant` panics with index out of range — pre-existing test bug.
- `TestMultipleIconThumbnailInstancesShareCanvasImages` fails — pre-existing test assertion issue.
- The systray error (`dbus-launch not found`) is expected in headless/container environments and does not affect app functionality.
- XGB Xauthority warnings are cosmetic and safe to ignore.

### Key gotchas

- The `eng.traineddata` file (`internal/assets/tessdata/eng.traineddata`) is gitignored and must be downloaded before building: `curl -sL -o internal/assets/tessdata/eng.traineddata https://github.com/tesseract-ocr/tessdata/raw/main/eng.traineddata`
- The `-buildvcs=false` flag is needed when building inside Docker (no git context).
- The `GOFLAGS="-tags=gocv_specific_modules"` env var is required (set in `devcontainer.json`).
- OpenCV 4.6+ is required; Ubuntu's system package is too old — the devcontainer builds it from source.
