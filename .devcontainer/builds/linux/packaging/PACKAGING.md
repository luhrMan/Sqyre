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

1. **Reuse host OpenCV/Tesseract/Leptonica (e.g. devcontainer, same as AppImage)**  
   When building inside the devcontainer (or any host with OpenCV in `/usr/local` and Tesseract/Leptonica from apt), run the prepare script once, then use the “host-deps” manifest so the Flatpak build reuses those libs instead of building OpenCV from source:

   ```bash
   .devcontainer/builds/linux/packaging/flatpak/prepare-flatpak-deps.sh
   flatpak-builder --user --force-clean build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.with-host-deps.yml
   ```

2. **Build Leptonica, Tesseract, and OpenCV from source in the manifest**  
   Use the full manifest `com.sqyre.app.yml`, which adds modules that build those dependencies. Slower but works without a pre‑built host (e.g. in CI).

### Build (once dependencies are available)

From the **repository root**:

**Option A – with host deps (devcontainer):**
```bash
.devcontainer/builds/linux/packaging/flatpak/prepare-flatpak-deps.sh
flatpak-builder --user --force-clean build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.with-host-deps.yml
```

**Option B – full manifest (builds OpenCV etc. in Flatpak):**
```bash
flatpak-builder --user --force-clean build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
```

**Install** (then run from menu or CLI):

```bash
flatpak-builder --user --install build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
flatpak run --user com.sqyre.app
```

**Run without installing** (from the build directory; do not pass `--user` with `--run`):

```bash
flatpak-builder --run build-dir .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml sqyre
```

**Run on the host (e.g. from a devcontainer with no display):**  
Export the build to a single-file bundle, copy it to your host, then install and run there:

```bash
# In devcontainer (from repo root), after a successful build:
.devcontainer/builds/linux/packaging/flatpak/export-bundle.sh

# Copy the generated .flatpak file to your host, then on the host:
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.freedesktop.Platform//25.08   # one-time, if needed
flatpak install --user com.sqyre.app.flatpak
flatpak run com.sqyre.app
```

The manifest is at `flatpak/com.sqyre.app.yml`. It expects the app to be built from `./cmd/sqyre` and uses metadata from `flatpak/com.sqyre.app.desktop` and `flatpak/com.sqyre.app.appdata.xml`.

### App icon and appstream compose (e.g. Nix)

If `flatpak-builder` fails with **appstreamcli compose** errors (`file-read-error`, `filters-but-no-output`) — for example when building under **Nix** — generate a 256×256 PNG from the SVG and commit it so the build uses the PNG instead of the SVG for the icon:

```bash
.devcontainer/builds/linux/packaging/flatpak/generate-app-icon-png.sh
# Then commit internal/assets/icons/sqyre-256.png and rebuild.
```

You need `rsvg-convert` (librsvg) or `convert` (ImageMagick) to run the script.

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
