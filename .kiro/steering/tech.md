---
inclusion: always
---

# Technology Stack

## Language & Runtime

- Go 1.23.0+
- Module name: `Squire`

## Core Libraries

- **Fyne v2.6.0** - Cross-platform GUI framework
- **Robotgo v1.0.0-rc2.1** - Mouse/keyboard automation and screen capture
- **GoCV v0.41.0** - OpenCV bindings for computer vision and image matching
- **Gosseract v2.4.1** - Tesseract OCR bindings for text recognition
- **gohook** - Global hotkey detection

## Build System

Standard Go toolchain with platform-specific dependencies.

### Linux Build

```bash
# Install system dependencies
sudo apt install tesseract-ocr libgl1-mesa-dev libx11-dev libx11-xcb-dev \
  libxtst-dev libxcursor-dev libxrandr-dev libxinerama-dev g++ clang \
  libtesseract-dev libxxf86vm-dev libxkbcommon-x11-dev golang-go cmake

# Install OpenCV via GoCV
go get -u -d gocv.io/x/gocv
cd $GOPATH/pkg/mod/gocv.io/x/gocv@v0.42.0
make install

# Build application
go build ./cmd/sqyre
```

### Windows Build (MSYS2)

Requires MSYS2 with mingw64 packages: mingw-w64-x86_64-toolchain, gcc, opencv, zlib, tesseract-ocr, leptonica.

Download English tessdata and set `TESSDATA_PREFIX=C:\msys64\mingw64\share\tessdata`.

### Cross-Platform Packaging

Uses `fyne-cross` for building platform-specific binaries. Configuration in `FyneApp.toml`.

## Common Commands

```bash
# Run application
go run ./cmd/sqyre

# Build binary
go build -o sqyre ./cmd/sqyre

# Install dependencies
go mod download

# Update dependencies
go get -u ./...
go mod tidy
```

## Configuration Files

- `internal/config/config.yaml` - Runtime configuration and macro storage
- `FyneApp.toml` - Fyne application metadata (icon, version, app ID)
