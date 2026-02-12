# Packaging Sqyre for Linux (Flatpak and AppImage)

Sqyre is a Fyne app that uses **CGO** for OpenCV (gocv) and Tesseract (gosseract). Packaging therefore requires a build environment where these libraries (and their dev packages) are available.

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
flatpak-builder --user --force-clean build-dir packaging/com.sqyre.app.yml
```

Install and run:

```bash
flatpak-builder --user --install --force-clean build-dir packaging/com.sqyre.app.yml
flatpak run --user com.sqyre.app
```

The manifest is at `packaging/com.sqyre.app.yml`. It expects the app to be built from `./cmd/sqyre` and uses metadata from `packaging/com.sqyre.app.desktop` and `packaging/com.sqyre.app.appdata.xml`.

---

## AppImage

### Prerequisites

- A **Linux build environment** with:
  - Go 1.24+
  - OpenCV and Tesseract (and Leptonica) dev libraries — same as in the main [README](../README.md) “Linux (non-NixOS)” section (e.g. `libtesseract-dev`, and OpenCV from gocv’s `make install` or distro packages).
- **appimage-builder**  
  Install e.g. with:
  ```bash
  pip3 install appimage-builder
  ```
  Or use the official [AppImage from releases](https://github.com/AppImageCrafters/appimage-builder/releases).  
  You also need `patchelf`, `squashfs-tools`, and other dependencies listed in the [appimage-builder docs](https://appimage-builder.readthedocs.io/en/stable/intro/install.html).

### Build

From the **repository root**:

```bash
appimage-builder --recipe .devcontainer/builds/linux/packaging/AppImageBuilder.yml
```

This runs the recipe’s `script` (builds the binary with `go build ./cmd/sqyre` and installs it and the desktop/icon into the AppDir), then bundles dependencies and produces an AppImage. The output file name is set in the recipe (e.g. `Sqyre-0.5.0-x86_64.AppImage`).

### Tesseract data (OCR)

For OCR to work, Tesseract needs language data (e.g. `eng.traineddata`). If the host has it in a standard path (e.g. `/usr/share/tessdata`), the bundled app may still look there at runtime. For a fully self-contained AppImage, you can copy `eng.traineddata` into the AppDir in the recipe (e.g. into `usr/share/tessdata/`) and set `TESSDATA_PREFIX` in the runtime environment in the recipe if needed.

---

## Summary

| Format   | Build from        | Main requirement                          |
|----------|-------------------|-------------------------------------------|
| **Flatpak** | Repo root, manifest in `packaging/` | OpenCV + Tesseract (and Leptonica) in build env or as manifest modules |
| **AppImage** | Repo root, recipe in `packaging/`   | OpenCV + Tesseract on host + appimage-builder |

Both assume the application entrypoint is at **`./cmd/sqyre`** (see main README for building that target).
