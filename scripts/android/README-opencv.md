# OpenCV for Android

Build OpenCV (and optional opencv_contrib) for Android ABIs with the NDK and CMake. Based on [this gist](https://gist.github.com/ogero/c19458cf64bd3e91faae85c3ac887481), adapted for Linux and OpenCV 4.x.

- **OpenCV 4.10.0** (gocv-compatible)
- One build per ABI: `armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`
- CMake + Ninja; contrib optional via `OPENCV_EXTRA_MODULES_PATH`

## Docker (recommended)

From the repo root:

```bash
docker build -f scripts/android/Dockerfile.opencv-android \
  -t opencv-android:local \
  scripts/android
```

Reuse the devcontainer image to skip reinstalling the NDK:

```bash
docker tag <devcontainer-image> sqyre-dev:latest
docker build -f scripts/android/Dockerfile.opencv-android \
  --build-arg BASE_IMAGE=sqyre-dev:latest \
  -t opencv-android:local \
  scripts/android
```

Single ABI:

```bash
docker build -f scripts/android/Dockerfile.opencv-android \
  --build-arg ABIS=arm64-v8a \
  -t opencv-android:local \
  scripts/android
```

Extract artifacts:

```bash
docker create --name opencv-android opencv-android:local
docker cp opencv-android:/opt/opencv/android/opencv-android-sdk ./opencv-android-sdk
docker rm opencv-android
```

Inside the image: `/opt/opencv/android/install_<abi>/` and SDK layout under `/opt/opencv/android/opencv-android-sdk/`.

## Host build

1. Install Android NDK (e.g. r25c), CMake, Ninja.
2. Set `ANDROID_NDK` to the NDK root.
3. Run `scripts/android/build-opencv-android.sh`.

| Variable | Default | Description |
|----------|---------|-------------|
| `ANDROID_NDK` | `/opt/android-ndk` | NDK root |
| `ANDROID_API_LEVEL` | `21` | Minimum API level |
| `OPENCV_VERSION` | `4.10.0` | OpenCV tag |
| `CONTRIB_VERSION` | `4.10.0` | opencv_contrib tag |
| `ABIS` | all four ABIs | Comma-separated list |
| `USE_CONTRIB` | `1` | Set `0` to skip contrib |
| `BUILD_ROOT` | `/opt/opencv/android` | Source, build, install root |

## Output layout

```
opencv-android-sdk/
  native/libs/<abi>/libopencv_*.so
  native/jni/include/opencv2/...
```

Matches the [OpenCV Android SDK](https://github.com/opencv/opencv/wiki/Custom-OpenCV-Android-SDK-and-AAR-package-build) convention.
