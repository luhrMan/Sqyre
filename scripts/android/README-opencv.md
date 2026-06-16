# OpenCV for Android (from source)

Build OpenCV (and optional opencv_contrib) for Android ABIs using the Android NDK and CMake, following the approach described in [this gist](https://gist.github.com/ogero/c19458cf64bd3e91faae85c3ac887481), adapted for **Linux** and **OpenCV 4.x**.

## Differences from the gist

- **Platform**: Linux + NDK toolchain (no MinGW; the gist used Windows + MinGW).
- **OpenCV**: 4.10.0 (compatible with gocv; the gist used 3.4.1).
- **Build**: CMake + Ninja, one build per ABI (`armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`).
- **Options**: Same spirit as the gist: `BUILD_JAVA=OFF`, `BUILD_SHARED_LIBS=ON`, `BUILD_*_EXAMPLES/TESTS/DOCS=OFF`, optional contrib via `OPENCV_EXTRA_MODULES_PATH`.

## Requirements

- Docker (for the image), or on the host: Android NDK, CMake, Ninja, and the script deps.

## Build with Docker (recommended)

From the **repository root**:

```bash
docker build -f scripts/android/Dockerfile.opencv-android \
  -t opencv-android:local \
  scripts/android
```

Build time is long (download OpenCV + contrib, then compile for four ABIs).

### Reuse devcontainer to reduce build time

If you already have the devcontainer image (e.g. from opening the project in a dev container), tag it and use it as the base so this image reuses NDK and build deps instead of reinstalling them:

```bash
# After the devcontainer has been built, tag it (use the image ID or name from docker images)
docker tag <your-devcontainer-image> sqyre-dev:latest

# Build OpenCV for Android using that base (skips NDK + apt layer)
docker build -f scripts/android/Dockerfile.opencv-android \
  --build-arg BASE_IMAGE=sqyre-dev:latest \
  -t opencv-android:local \
  scripts/android
```

To build only one ABI:

```bash
docker build -f scripts/android/Dockerfile.opencv-android \
  --build-arg ABIS=arm64-v8a \
  -t opencv-android:local \
  scripts/android
```

Artifacts inside the image (under `/opt/opencv/android/`):

- Per-ABI: `/opt/opencv/android/install_<abi>/` (libs + headers).
- SDK-style: `/opt/opencv/android/opencv-android-sdk/native/libs/<abi>/` and `.../jni/include/`.

Copy them out:

```bash
docker create --name opencv-android opencv-android:local
docker cp opencv-android:/opt/opencv/android/opencv-android-sdk ./opencv-android-sdk
docker rm opencv-android
```

## Build on the host (no Docker)

1. Install Android NDK (e.g. r25c), CMake, Ninja.
2. Set `ANDROID_NDK` to the NDK root (e.g. `/opt/android-ndk` or `$HOME/Android/Sdk/ndk/25.2.9519653`).
3. Run:

```bash
scripts/android/build-opencv-android.sh
```

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `ANDROID_NDK` | `/opt/android-ndk` | NDK root (must contain `build/cmake/android.toolchain.cmake`) |
| `ANDROID_API_LEVEL` | `21` | Minimum Android API level |
| `OPENCV_VERSION` | `4.10.0` | OpenCV tag |
| `CONTRIB_VERSION` | `4.10.0` | opencv_contrib tag (must match OpenCV) |
| `ABIS` | `armeabi-v7a,arm64-v8a,x86,x86_64` | Comma-separated ABIs |
| `USE_CONTRIB` | `1` | Set to `0` to skip contrib |
| `BUILD_ROOT` | `/opt/opencv/android` | Source, build, and install directory (compiled output lives here) |

## Output layout (SDK-style)

```
opencv-android-sdk/
  native/
    libs/
      armeabi-v7a/   libopencv_*.so
      arm64-v8a/     ...
      x86/           ...
      x86_64/        ...
    jni/
      include/       opencv2/ ...
```

This layout matches what the [OpenCV Android build](https://github.com/opencv/opencv/wiki/Custom-OpenCV-Android-SDK-and-AAR-package-build) and the [reference gist](https://gist.github.com/ogero/c19458cf64bd3e91faae85c3ac887481) describe (shared libs under `libs/<abi>`, headers under `jni/include`).
