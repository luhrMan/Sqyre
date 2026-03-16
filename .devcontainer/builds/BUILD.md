# Builds overview

This directory contains build configs for Linux (devcontainer + Nix + AppImage), Android (fyne-cross + optional OpenCV Android), and Windows (fyne-cross + static OpenCV/Tesseract).

**Dev container targets:** The main devcontainer Dockerfile has two build targets. Default is `devcontainer` (no Android NDK). To include the Android NDK, set `"target": "with-android"` in `.devcontainer/devcontainer.json` under `build` and rebuild. See `.devcontainer/README.md`.

## Versions

Version ARGs (e.g. `GO_VERSION`, `OPENCV_VERSION`, `NDK_REV`) are defined in each Dockerfile. When bumping, update the main `.devcontainer/Dockerfile` and any other Dockerfiles that use the same tool (Android/Windows images). OpenCV build scripts live under [opencv/](opencv/); Linux, Android, and Windows each use the script for their platform.

## Reusing builds to reduce time

| Build | Reuses | How |
|-------|--------|-----|
| **Android OpenCV image** | Devcontainer (NDK + apt + cmake/ninja) | Build the devcontainer first, tag it (e.g. `sqyre-dev:latest`), then build with `--build-arg BASE_IMAGE=sqyre-dev:latest`. See [opencv/android/README.md](opencv/android/README.md). |
| **Linux devcontainer** | — | Base for the workspace; no other image is built from it unless you use it as `BASE_IMAGE` for Android OpenCV. |
| **Windows image** | — | Uses Debian + MinGW and static libs; no overlap with Linux/Android. |
| **Android fyne-cross image** | — | Based on `fyneio/fyne-cross-images:android`; only upgrades Go. |

## What cannot be shared

- **OpenCV binaries**: Linux (4.6, host), Android (4.10, per-ABI), and Windows (static MinGW) are different toolchains and targets. Only the *environment* (NDK, deps) can be reused for Android OpenCV by using the devcontainer as base.
- **Windows** uses a different distro and MinGW; no layer sharing with Linux/Android.
- **Nix** (`linux/nix/`) is independent of Docker.
