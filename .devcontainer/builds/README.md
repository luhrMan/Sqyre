# Build images

## Windows cross-compile (Dockerfile.windows-amd64)

Docker image for **cross-compiling Sqyre to Windows** with **OpenCV4** and CGO dependencies (gocv, gosseract) available inside the container. Extends `fyneio/fyne-cross-images:windows` with a MinGW sysroot so the Go build can link against OpenCV 4.x, Tesseract, and Leptonica for Windows.

### What's in the image

- **OpenCV 4** (and Tesseract/Leptonica) for Windows, from MSYS2 MinGW packages, extracted into `/usr/local/mingw64` (no Wine). Sqyre's gocv and gosseract code compiles and links against these.
- **Stage 1**: Python 3.12 + `msys2dl` — downloads MSYS2 packages `opencv`, `zlib`, `tesseract-ocr`, `leptonica` (and deps) plus `eng.traineddata`. Extracts to `/usr/local/mingw64`.
- **Stage 2**: fyne-cross Windows base + the sysroot; MinGW GCC/G++ and CGO flags so the compiler finds OpenCV4 and the rest. An entrypoint forces CC/CXX to MinGW (GNU ld) so fyne-cross doesn't override with Zig's lld (which can fail with CGO).

The image adds symlinks from MSYS2's `libopencv_*-413.dll.a` to `libopencv_*4130.dll.a` so gocv's `-lopencv_core4130` etc. resolve correctly.

### Build the image (from repo root)

```bash
# From repository root
docker build -f .devcontainer/builds/Dockerfile.windows-amd64 -t fyne-cross-windows:local .
# Or use the copy under windows/
docker build -f .devcontainer/builds/windows/Dockerfile.windows-amd64 -t fyne-cross-windows:local .
```

### Cross-compile Sqyre for Windows

```bash
fyne-cross windows -image fyne-cross-windows:local --app-id com.sqyre.app ./cmd/sqyre
```

Or use the helper script (from repo root):

```bash
.devcontainer/builds/windows/build.sh
```

`build.sh` builds the image then runs `fyne-cross windows` with that image and `./cmd/sqyre`.

### OpenCV compile errors

The image installs a custom **opencv4.pc** so `pkg-config opencv4` (used by gocv) returns the correct MinGW include/lib paths and `-lopencv_*4130` library names. If you still see OpenCV-related errors:

- **opencv2/opencv.hpp not found** — The custom `opencv4.pc` sets `Cflags: -I${includedir}/opencv4`; ensure the image was rebuilt after adding that step.
- **cannot find -lopencv_core4130** — The symlinks in the image map MSYS2’s `libopencv_*-413.dll.a` to `*4130.dll.a`. Rebuild the image so the sysroot and symlinks are up to date.
- **undefined reference (C++ stdlib)** — The entrypoint adds the MinGW C++ include path first so `<mutex>` comes from the host toolchain. Confirm `entrypoint-mingw.sh` is unchanged.

If problems persist, try building with gocv’s customenv tag so only the container’s CGO_* env are used:  
`fyne-cross windows -image fyne-cross-windows:local --app-id com.sqyre.app -tags customenv,gocv_specific_modules ./cmd/sqyre`
