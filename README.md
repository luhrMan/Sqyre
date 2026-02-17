# What is it

Sqyre is a Macro Builder, written using GO, with a few notable libraries:

- Fyne (GUI)
- Robotgo (Automation)
- Gosseract aka Tesseract (OCR)
- GoCV aka OpenCV (Computer Vision)

The structure of the fyne `widget.Tree`:

- (Root) 1 Loop Action
- (Branch) Action with SubAction (Advanced Actions)
    - `Loop`
    - `Image Search`
    - `OCR`
- (Leaf) Action
    - `Click`: click the mouse where cursor is at
    - `Move`: move the mouse to specific coordinates
    - `Key`: Set a key state Up/Down
    - `Wait`: Wait for time set in milliseconds

# Main Screen
<img width="2562" height="1362" alt="Screenshot from 2026-01-13 13-09-30" src="https://github.com/user-attachments/assets/53acf1a0-bc89-43d9-a7ab-856b46c3be63" />

# ImageSearch in action
![sqyre-imagesearch](https://github.com/user-attachments/assets/1a0fc8f4-06bb-4667-bb49-b1c4b2d5b508)

# Why

fuck all that clicking

# BUILD INSTRUCTIONS

## Linux

### 1. Install dependencies

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

### 2. Install OpenCV

Squire uses **gocv**; OpenCV **≥ 4.6** is required. Either:

- **Option A — Build via gocv (from repo root):**
  ```bash
  go get -u -d gocv.io/x/gocv
  cd $(go env GOPATH)/pkg/mod/gocv.io/x/gocv@v0.43.0
  make install
  ```
- **Option B — Build from source** (e.g. OpenCV 4.6+ with `core`, `imgproc`, `imgcodecs`; see `.devcontainer/Dockerfile` for a reference CMake setup).

### 3. Build

From the repository root:

```bash
go build -o sqyre ./cmd/sqyre
./sqyre
```

For **Flatpak** or **AppImage** packaging, see [.devcontainer/builds/linux/packaging/PACKAGING.md](.devcontainer/builds/linux/packaging/PACKAGING.md).

---

## Windows

### Recommended: Docker cross-compile (from Linux or WSL)

Build a standalone Windows `.exe` with OpenCV and Tesseract statically linked (no DLLs). Requires **Docker** and **fyne-cross**.

From the **repository root**:

```bash
# Build the Windows image and compile (output: .devcontainer/builds/windows/output/Sqyre.exe)
bash .devcontainer/builds/windows/build.sh
```

If you're not in the dev container, ensure `fyne-cross` is installed (`go install github.com/fyne-io/fyne-cross@latest`) and run:

```bash
docker build -f .devcontainer/builds/windows/docker/Dockerfile.windows-amd64 -t fyne-cross-windows:local .
fyne-cross windows -image fyne-cross-windows:local --app-id com.sqyre.app ./cmd/sqyre
```

The built executable is at `.devcontainer/builds/windows/output/Sqyre.exe`.

### Native Windows (MSYS2)

Using the **mingw64** shell in [MSYS2](https://www.msys2.org/):

1. **Install packages**
   - [mingw-w64-x86_64-toolchain](https://packages.msys2.org/groups/mingw-w64-x86_64-toolchain)
   - [mingw-w64-x86_64-gcc](https://packages.msys2.org/package/mingw-w64-x86_64-gcc)
   - [mingw-w64-x86_64-opencv](https://packages.msys2.org/package/mingw-w64-x86_64-opencv)
   - [mingw-w64-x86_64-zlib](https://packages.msys2.org/package/mingw-w64-x86_64-zlib)
   - [mingw-w64-x86_64-tesseract-ocr](https://packages.msys2.org/package/mingw-w64-x86_64-tesseract-ocr)
   - [mingw-w64-x86_64-leptonica](https://packages.msys2.org/package/mingw-w64-x86_64-leptonica)
   - Optional: [mingw-w64-x86_64-go](https://packages.msys2.org/package/mingw-w64-x86_64-go?repo=mingw64) if you want Go inside MSYS2.

2. **Tesseract English data**
   - Download [eng.traineddata](https://github.com/tesseract-ocr/tessdata/blob/main/eng.traineddata) and put it in `C:\msys64\mingw64\share\tessdata`.
   - In mingw64: `export TESSDATA_PREFIX=C:/msys64/mingw64/share/tessdata`

3. **Optional: set Go env (if using MSYS2 Go)**
   - `export GOROOT=/mingw64/lib/go`
   - `export GOPATH=/mingw64`

4. **Build** from the repo (in mingw64): `go build -o sqyre.exe ./cmd/sqyre`

To use the MSYS2 shell in VS Code: [integrate MSYS2 with VS Code](https://stackoverflow.com/questions/45836650/how-do-i-integrate-msys2-shell-into-visual-studio-code-on-windows).
