# Android builds (fyne-cross)

Builds Sqyre for Android using [fyne-cross](https://github.com/fyne-io/fyne-cross).

## Architectures

- **arm64** – 64-bit ARM (most current devices)
- **arm** – 32-bit ARM (ARMv7)
- **amd64** – x86_64 (emulators, some tablets)
- **386** – x86 (emulators)

## Local build

From the repository root:

**1. Build the custom fyne-cross Android image** (provides Go 1.26; the official image has 1.24 and go.mod requires ≥1.25):

```bash
docker build -f .devcontainer/builds/android/docker/Dockerfile.android \
  -t fyne-cross-android:local .
```

**2. Build the APKs:**

```bash
# All architectures (default)
.devcontainer/builds/android/build.sh

# Or limit architectures
ANDROID_ARCHES=arm64,arm .devcontainer/builds/android/build.sh
```

Requires Docker and fyne-cross:

```bash
go install github.com/fyne-io/fyne-cross@latest
```

Output: `fyne-cross/dist/android-<arch>/` and `.devcontainer/builds/android/output/`.

## CI

The GitHub workflow runs `build-android` on push to `main`, producing versioned APKs and attaching them to the release.

## Notes

- The project uses a **custom** fyne-cross Android image (`Dockerfile.android`) that upgrades Go to 1.26 so it satisfies go.mod (≥1.25). If the app uses desktop-only dependencies (e.g. gocv, robotgo), add build tags or constraints so those packages are excluded when building for Android.
- Android cross-compilation runs only on **amd64** hosts (e.g. GitHub’s `ubuntu-22.04` runners).
