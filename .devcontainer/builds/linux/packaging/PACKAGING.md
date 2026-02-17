# Packaging Sqyre for Linux (Flatpak and AppImage)

Sqyre is a Fyne app that uses **CGO** for OpenCV (gocv) and Tesseract (gosseract). Packaging therefore requires a build environment where these libraries (and their dev packages) are available.

Layout:

- **`flatpak/`** — Flatpak manifest, desktop file, and appdata
- **`appimage/`** — AppImage recipe, build script, and desktop file

---

## Flatpak

### Prerequisites

- `flatpak` and `flatpak-builder`
- Flathub remote and SDK with Golang extension:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.golang//25.08
```

### OpenCV and Tesseract in Flatpak

The **freedesktop SDK does not include OpenCV or Tesseract**. You have two options:

1. **Add dependency modules to the manifest**  
   Add modules that build **Leptonica**, **Tesseract**, and **OpenCV** from source before the Sqyre module, and set `PKG_CONFIG_PATH` / `CGO_*` so the Go build finds them. Examples for Tesseract/Leptonica:
   - [TextSnatcher manifests](https://github.com/RajSolai/TextSnatcher/tree/master/manifests)  
   Update the SHA256 hashes in the manifest (e.g. run `sha256sum` on the downloaded tarballs).

2. **Use a custom SDK or build on a host that has them**  
   If you build on a system with OpenCV and Tesseract installed (e.g. from your distro), you can try pointing the Flatpak build at them; this is less reproducible and may require a custom runtime/SDK that includes those libs.

### Build (once dependencies are available)

From the **repository root**:

```bash
flatpak-builder --user --force-clean build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
```

Install and run:

```bash
flatpak-builder --user --install --force-clean build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
flatpak run --user com.sqyre.app
```

The manifest is at `flatpak/com.sqyre.app.yml`. It expects the app to be built from `./cmd/sqyre` and uses metadata from `flatpak/com.sqyre.app.desktop` and `flatpak/com.sqyre.app.appdata.xml`.

---

## AppImage

### Prerequisites

- A **Linux build environment** with:
  - Go 1.24+
  - OpenCV and Tesseract (and Leptonica) dev libraries — same as in the main README “Linux (non-NixOS)” section (e.g. `libtesseract-dev`, and OpenCV from gocv’s `make install` or distro packages).
- **appimage-builder**  
  Install e.g. with:
  ```bash
  pip3 install appimage-builder
  ```
  Or use the official [AppImage from releases](https://github.com/AppImageCrafters/appimage-builder/releases).  
  You also need `patchelf`, `squashfs-tools`, and other dependencies listed in the [appimage-builder docs](https://appimage-builder.readthedocs.io/en/stable/intro/install.html).

### Build

To keep `sqyre.AppDir` and the AppImage under `appimage/`, run from the **repository root**:

```bash
.devcontainer/builds/linux/packaging/appimage/build-appimage.sh
```

Or from the appimage directory: `cd .devcontainer/builds/linux/packaging/appimage && ./build-appimage.sh` (run `chmod +x build-appimage.sh` once). `sqyre.AppDir`, `appimage-build`, and the .AppImage file are created inside `appimage/` (see `.gitignore`).

### Tesseract data (OCR)

The recipe copies `eng.traineddata` from the host’s `/usr/share/tessdata/` into `sqyre.AppDir` when present, and sets `TESSDATA_PREFIX` at runtime so OCR works in the AppImage.

---

## Summary

| Format    | Build from | Main requirement |
|-----------|------------|-------------------|
| **Flatpak**  | Repo root, manifest in `flatpak/` | OpenCV + Tesseract (and Leptonica) in build env or as manifest modules |
| **AppImage** | Repo root, script in `appimage/`   | OpenCV + Tesseract on host + appimage-builder |

Both assume the application entrypoint is at **`./cmd/sqyre`** (see main README for building that target).
