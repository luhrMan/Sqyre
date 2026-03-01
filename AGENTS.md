# Agents

## Cursor Cloud specific instructions

### Architecture

Sqyre is a standalone desktop GUI macro builder (Go 1.24 + Fyne UI toolkit). No external services, databases, or APIs. See `README.md` for full build instructions.

### Build workflow (Docker-based)

The devcontainer Dockerfile (`.devcontainer/Dockerfile`) is the canonical build environment. It builds OpenCV 4.6.0 from source, installs Tesseract OCR, and all required X11/OpenGL dev libs on Ubuntu 22.04.

**Build the Docker image** (one-time, ~5 min):
```
sudo docker build -f .devcontainer/Dockerfile -t sqyre-dev:latest .
```

**Compile the binary** inside the container:
```
sudo docker run --rm -v /workspace:/workspace -w /workspace \
  -e GOFLAGS="-tags=gocv_specific_modules" \
  sqyre-dev:latest \
  sh -c "go build -buildvcs=false -tags gocv_specific_modules -o /workspace/sqyre ./cmd/sqyre"
```

**Run tests** (requires Xvfb for X11-dependent packages):
```
sudo docker run --rm -v /workspace:/workspace -w /workspace \
  -e GOFLAGS="-tags=gocv_specific_modules" -e SQYRE_NO_HOOK=1 -e DISPLAY=:99 \
  sqyre-dev:latest \
  sh -c "apt-get update -qq && apt-get install -y -qq xvfb >/dev/null 2>&1 && \
    Xvfb :99 -screen 0 1920x1080x24 &>/dev/null & sleep 1 && \
    go test -buildvcs=false -tags gocv_specific_modules -count=1 ./..."
```

### Running the binary on the VM

The binary links against shared libraries from the Docker container (OpenCV 4.6 .so files, libtesseract.so.4). These are extracted to `/usr/local/lib/` during setup. To run:
```
export DISPLAY=:99 LD_LIBRARY_PATH=/usr/local/lib SQYRE_NO_HOOK=1
Xvfb :99 -screen 0 1920x1080x24 &>/dev/null &
./sqyre
```

Set `SQYRE_NO_HOOK=1` in headless environments to skip the global keyboard hook (requires a real X display with XInput).

### Gotchas

- `eng.traineddata` (Tesseract model) is gitignored. The update script downloads it to `internal/assets/tessdata/` if missing. Without it, `go build` fails on the embed directive.
- `testdata/db.yaml` is needed by repository tests. It must be a copy of `testdata/config.yaml`. The update script creates it if missing.
- `-buildvcs=false` is required when building inside Docker (git ownership mismatch).
- The `GOFLAGS="-tags=gocv_specific_modules"` build tag is set in `devcontainer.json` `containerEnv` — always pass it for build/test.
- 2 pre-existing test failures: `TestGetVariants/Legacy_icon_without_variant` (panic) and `TestMultipleIconThumbnailInstancesShareCanvasImages`.
- Windows cross-compile via `.devcontainer/builds/windows/build.sh` requires `fyne-cross` and Docker.
