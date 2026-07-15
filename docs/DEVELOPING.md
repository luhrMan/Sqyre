# Developing Sqyre

## Dev container (recommended)

Open the repository in the dev container (`.devcontainer/`). It is **Rust-migration focused**: Rust 1.92, clang, Tesseract/Leptonica, and Linux GUI link deps (no OpenCV source build, Go, or packaging tooling). For the Go/Fyne app, use a host toolchain or the scripts under `scripts/` / `docs/DEVELOPING.md` native-deps section.

From the repo root:

```bash
make linux          # ./bin/sqyre
make windows        # bin/windows-amd64/sqyre.exe (fyne-cross in Docker)
make appimage       # bin/*.AppImage
make tessdata       # download eng.traineddata for OCR
```

Run `make help` for matprofile variants (`windows-matprofile`, `appimage-matprofile`).

---

## Make targets

| Target | Output |
|--------|--------|
| `linux` | `bin/sqyre` |
| `windows` | `bin/windows-amd64/sqyre.exe` |
| `appimage` | `bin/Sqyre-*.AppImage` |
| `tessdata` | Tesseract trained data via `scripts/download-tessdata.sh` |
| `*-matprofile` | Same as above with `matprofile` build tag |

Set `BUILD_TAGS` to override tags (default: `gocv_specific_modules`).

---

## Native dependencies

Sqyre uses **CGO** for OpenCV (gocv) and Tesseract (gosseract). OpenCV **≥ 4.6** is required.

| Resource | Purpose |
|----------|---------|
| [.devcontainer/Dockerfile](../.devcontainer/Dockerfile) | Rust migration image (clang, Tesseract; not OpenCV) |
| [scripts/linux/build-opencv-linux.sh](../scripts/linux/build-opencv-linux.sh) | Build OpenCV for Go/gocv on Linux |
| [scripts/windows/build-opencv-windows.sh](../scripts/windows/build-opencv-windows.sh) | Build OpenCV for Windows cross-compile |
| [scripts/android/README-opencv.md](../scripts/android/README-opencv.md) | OpenCV for Android ABIs |

---

## Manual setup (without dev container)

These paths are maintained less actively than the dev container. Prefer the container when possible.

### Linux

```bash
sudo apt install -y \
  build-essential pkg-config cmake golang-go \
  tesseract-ocr libtesseract-dev libleptonica-dev \
  libgl1-mesa-dev libglvnd-dev libglfw3-dev \
  libxkbcommon-dev libxkbcommon-x11-dev \
  libx11-dev libx11-xcb-dev libxext-dev libxtst-dev \
  libxcursor-dev libxrandr-dev libxinerama-dev \
  libxxf86vm-dev libxt-dev \
  libjpeg-dev libpng-dev libtiff-dev libwebp-dev libopenjp2-7-dev
```

Build or install OpenCV to match gocv (see Dockerfile and `build-opencv-linux.sh`), then:

```bash
go build -tags gocv_specific_modules -o sqyre ./cmd/sqyre
```

### Windows (MSYS2 mingw64)

In the [MSYS2](https://www.msys2.org/) mingw64 shell, install toolchain, OpenCV, Tesseract, and Leptonica from MSYS2 packages (e.g. `mingw-w64-x86_64-opencv`, `mingw-w64-x86_64-tesseract-ocr`).

1. Place [eng.traineddata](https://github.com/tesseract-ocr/tessdata/blob/main/eng.traineddata) in `C:\msys64\mingw64\share\tessdata`.
2. Set `TESSDATA_PREFIX=C:/msys64/mingw64/share/tessdata`.
3. Build: `go build -o sqyre.exe ./cmd/sqyre`

Or cross-compile from Linux with `make windows`.

---

## Tests

**Headless** (no display; uses `-tags=nohook` so the keyboard hook is not linked). Includes README screenshot golden checks (`TestDocsScreenshots`, `TestDemoWorkflowFrames`):

```bash
./scripts/test.sh
./scripts/test.sh -v ./internal/services/ -run TestExecute
```

**Global hook / Esc** (virtual framebuffer via `Xvfb`; links gohook):

```bash
./scripts/test-ui.sh
./scripts/test-ui.sh -run TestGUIEscape
```

Plain `go test` without these wrappers can segfault when `DISPLAY` is unset because of the native hook.

---

## README screenshots & demo GIF

Regenerate assets in `docs/images/`:

```bash
./scripts/generate-docs-media.sh
```

Requires `ffmpeg` for the demo GIF. Uses the same headless Fyne driver as `./scripts/test.sh` (no xvfb).

Verify committed PNGs match the current UI:

```bash
./scripts/test.sh -v ./ui/ -run 'TestDocsScreenshots|TestDemoWorkflowFrames'
```

Set `SQYRE_UPDATE_SCREENSHOTS=1` when intentionally updating golden images (the generate script sets this).

---

## GoCV `Mat` profiling

Build with the **`matprofile`** tag to track `gocv.Mat` allocations and detect leaks.

| Platform | Command |
|----------|---------|
| Linux | `go build -tags "gocv_specific_modules,matprofile" -o sqyre ./cmd/sqyre` |
| Windows (from dev container) | `make windows-matprofile` |

- Logs: `~/.sqyre/sqyre.log` (Windows: `%USERPROFILE%\.sqyre\sqyre.log`)
- pprof HTTP server on `127.0.0.1:6060` (or next free port 6061–6065); open **gocv.io/x/gocv.Mat** in the browser for stack traces
- `SQYRE_PPROF=0` disables pprof; `SQYRE_PPROF=127.0.0.1:9090` sets a fixed port

---

## Packaging

See [scripts/linux/packaging/PACKAGING.md](../scripts/linux/packaging/PACKAGING.md) for Flatpak and AppImage builds.
