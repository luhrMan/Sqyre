# Dev container

Default image: Linux build only (Go, OpenCV, Fyne, AppImage tooling). No Android NDK.

## Including Android tools

To add the Android NDK to the dev container (for Android builds or OpenCV-for-Android):

1. In `devcontainer.json`, set the build target to `with-android`:

   ```json
   "build": {
     "dockerfile": "Dockerfile",
     "context": "..",
     "target": "with-android"
   }
   ```

2. Rebuild the container (e.g. “Rebuild Container” in the command palette).

The image will then include the NDK at `/opt/android-ndk` and `ANDROID_NDK` will be set.
