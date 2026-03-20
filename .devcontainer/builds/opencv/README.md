# OpenCV builds

All OpenCV build logic for this project lives under **`builds/opencv/`**. Anything that needs to build OpenCV uses scripts from this folder. Compiled output is kept in a dedicated folder per platform inside the image (not `/tmp` or mixed into `/usr/local`):

| Platform | Script / image path | Compiled output in image |
|----------|---------------------|---------------------------|
| **Linux** (native) | `opencv/linux/build-opencv-linux.sh` | `/opt/opencv/linux/install` (lib, include, pkgconfig); env: `PKG_CONFIG_PATH`, `LD_LIBRARY_PATH` |
| **Android** (NDK, multi-ABI) | `opencv/android/` | `/opt/opencv/android/` (source, build per ABI, `opencv-android-sdk/`) |
| **Windows** (MinGW static) | `opencv/windows/build-opencv-windows.sh` | Build tree: `/opt/opencv/windows/`; install: sysroot (e.g. `/usr/local/mingw64-static`) |

Version ARGs (e.g. `OPENCV_VERSION`) are defined in each consumer Dockerfile or script. When bumping OpenCV, update the main `.devcontainer/Dockerfile`, the Android Dockerfile in `opencv/android/`, and the Windows script/env in `opencv/windows/` as needed.
