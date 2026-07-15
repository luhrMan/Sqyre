# Packaging Sqyre for Linux

**Note:** Local daily driver is Rust (`make` → `./bin/sqyre`). Release packaging below still builds the **legacy Go/Fyne** binary until CI/AppImage/Flatpak cut over — see [rust/MIGRATION.md](../../../rust/MIGRATION.md).

Sqyre packaging here is a Fyne app with **CGO** (OpenCV via gocv, Tesseract via gosseract). Needs a build environment where those libraries and dev headers are available.

| Path | Contents |
|------|----------|
| `scripts/linux/packaging/flatpak/` | Manifest (`com.sqyre.app.yml`), desktop file, appdata |
| `scripts/linux/packaging/appimage/` | AppImage recipe, build scripts, desktop file |

App entrypoint: `./cmd/sqyre`. For a plain binary first, see [docs/DEVELOPING.md](../../../docs/DEVELOPING.md).

---

## Flatpak

### Prerequisites

- `flatpak`, `flatpak-builder`, and **`elfutils`** (`eu-strip` — without it the finish step can fail)
- Flathub Platform/SDK **25.08** and the Golang extension

The **dev container** pre-installs user Flathub runtime/SDK when the image is built; skip manual `flatpak install` there unless you changed runtime versions.

On hosts without a system bus (minimal containers), use **`--user`** for remote add, install, and `flatpak-builder` (examples below use `--user`).

### flatpak-builder version

**Ubuntu 22.04** ships flatpak-builder **1.2.x**, which expects a legacy `appstream-compose` binary missing from current SDKs. Use **≥ 1.4.x** instead:

```bash
sudo scripts/linux/packaging/flatpak/install-flatpak-builder-1.4-on-jammy.sh
# Ensure /usr/local/bin is before /usr/bin in PATH
```

Newer distros often package 1.4.x directly. Check with `flatpak-builder --version`.

### OpenCV & Tesseract

The freedesktop SDK does **not** include OpenCV or Tesseract. The Sqyre manifest builds **Leptonica**, **Tesseract**, and **OpenCV** from source before the app module. First OpenCV build is slow; lower `MAKEFLAGS` / `NINJA_JOBS` in the manifest if the host runs out of memory.

Alternative references: [TextSnatcher manifests](https://github.com/RajSolai/TextSnatcher/tree/master/manifests).

### Build & install

Run from the **repository root**. Keep the build directory on the same filesystem as the repo (or pass `--state-dir` on the same disk):

```bash
flatpak-builder --user --force-clean \
  scripts/linux/packaging/flatpak/build-dir \
  scripts/linux/packaging/flatpak/com.sqyre.app.yml

flatpak-builder --user --install --force-clean \
  scripts/linux/packaging/flatpak/build-dir \
  scripts/linux/packaging/flatpak/com.sqyre.app.yml

flatpak run --user com.sqyre.app
```

The manifest enables network for `go mod download`. For offline builds, vendor modules and use `-mod=vendor` (see manifest comments).

---

## AppImage

### Prerequisites

- Linux build with Go 1.24+, OpenCV, Tesseract, and Leptonica (same as [DEVELOPING.md](../../../docs/DEVELOPING.md))
- [appimage-builder](https://appimage-builder.readthedocs.io/en/stable/intro/install.html) (`pip3 install appimage-builder` or release AppImage), plus `patchelf` and `squashfs-tools`

The dev container installs OpenCV under **`/opt/opencv/linux/install`**; the recipe bundles `.so` files from there (or falls back to `/usr/local/lib`).

### Build

From the repo root:

```bash
make appimage
# or: scripts/linux/packaging/appimage/build-appimage.sh
```

Output: **`bin/*.AppImage`**. `sqyre.AppDir` and build artifacts stay under `scripts/linux/packaging/appimage/`.

### Tesseract data

The recipe copies `eng.traineddata` from the host `/usr/share/tessdata/` when present and sets `TESSDATA_PREFIX` at runtime.

---

## Summary

| Format | Command | Main requirement |
|--------|---------|------------------|
| **Flatpak** | `flatpak-builder` + manifest | Manifest builds Leptonica, Tesseract, OpenCV; long first build |
| **AppImage** | `make appimage` | OpenCV + Tesseract on host + appimage-builder |
