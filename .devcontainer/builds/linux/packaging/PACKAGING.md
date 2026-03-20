# Packaging Sqyre for Linux (Flatpak and AppImage)

Sqyre is a Fyne app that uses **CGO** for OpenCV (gocv) and Tesseract (gosseract). Packaging therefore requires a build environment where these libraries (and their dev packages) are available.

Layout:

- **`.devcontainer/builds/linux/packaging/flatpak/`** — Flatpak manifest (`com.sqyre.app.yml`), desktop file, and appdata
- **`.devcontainer/builds/linux/packaging/appimage/`** — AppImage recipe, build script, and desktop file

---

## Flatpak

### Prerequisites

- `flatpak` and `flatpak-builder`
- **`elfutils`** on the host (provides `eu-strip`). Without it, the finish step can fail after a successful Go build with `Failed to execute child process "eu-strip"`. The devcontainer Dockerfile installs this with the Flatpak stage.
- Flathub remote and SDK with Golang extension:

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.golang//25.08
```

If **`Unable to connect to system bus`** appears (e.g. minimal containers), use a **user** installation instead:

```bash
flatpak --user remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak --user install -y flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.golang//25.08
```

Then pass **`--user`** to `flatpak-builder` as in the build commands below.

The **devcontainer Dockerfile** runs that user Flathub install during image build (after the `vscode` user is created), so a rebuilt dev image already has Platform, Sdk, and the Golang extension—**skip the manual `flatpak install` steps** in the container unless you changed runtime versions. Image build needs network for that layer.

### flatpak-builder version (AppStream / `appstream-compose`)

Check with `flatpak-builder --version`.

**Ubuntu 22.04 LTS** (jammy) ships **flatpak-builder 1.2.x** only. That series still expects a legacy **`appstream-compose`** helper inside the SDK sandbox. Current **org.freedesktop.Sdk** runtimes generally **do not** provide that binary, so the **finish** step can fail with `execvp appstream-compose: No such file or directory`. **flatpak-builder 1.4.x** uses **`appstreamcli compose`** on the host (AppStream ≥ 0.15.0). On jammy you need both **`appstream`** and the distro package **`appstream-compose`** (that package installs `/usr/libexec/appstreamcli-compose`; without it, `appstreamcli compose` exits with status 4 and Meson fails). The install script installs both.

**If you must stay on Ubuntu 22.04** (no newer distro), install **1.4.x** yourself:

- **Automated (recommended):** from the repo root, run  
  `sudo .devcontainer/builds/linux/packaging/flatpak/install-flatpak-builder-1.4-on-jammy.sh`  
  This builds [flatpak-builder 1.4.7](https://github.com/flatpak/flatpak-builder/releases) with Meson and installs to **`/usr/local`** (override with `PREFIX=...`). Put **`/usr/local/bin` before `/usr/bin`** in `PATH` so `flatpak-builder --version` reports 1.4.x, not 1.2.x.
- **Manual:** same dependencies as in that script, then download the release `.tar.xz`, `meson setup build --prefix=/usr/local -Dtests=false`, `meson compile -C build`, `sudo meson install -C build`.

On **newer distros**, prefer the packaged **flatpak-builder ≥ 1.4** from your repositories when available (e.g. Debian *trixie* [1.4.7](https://packages.debian.org/trixie/flatpak-builder)).

There is no Flathub “app” that replaces the host `flatpak-builder` CLI for manifest builds.

If you are **forced to keep 1.2.x**, workarounds are brittle (sandbox PATH and SDK layout); upgrading the builder as above is the reliable fix on jammy.

### OpenCV and Tesseract in Flatpak

The **freedesktop SDK does not include OpenCV or Tesseract**. You have two options:

1. **Add dependency modules to the manifest**  
   Add modules that build **Leptonica**, **Tesseract**, and **OpenCV** from source before the Sqyre module, and set `PKG_CONFIG_PATH` / `CGO_*` so the Go build finds them. Examples for Tesseract/Leptonica:
   - [TextSnatcher manifests](https://github.com/RajSolai/TextSnatcher/tree/master/manifests)  
   Update the SHA256 hashes in the manifest (e.g. run `sha256sum` on the downloaded tarballs).

2. **Use a custom SDK or build on a host that has them**  
   If you build on a system with OpenCV and Tesseract installed (e.g. from your distro), you can try pointing the Flatpak build at them; this is less reproducible and may require a custom runtime/SDK that includes those libs.

### Build (once dependencies are available)

From the **repository root**, use a **build directory on the same filesystem** as the repo (Flatpak’s default state dir is `.flatpak-builder` next to your sources; if you put `build-dir` on another mount, pass e.g. `--state-dir` on the same disk or you will get a “not on the same filesystem” error):

```bash
flatpak-builder --user --force-clean \
  .devcontainer/builds/linux/packaging/flatpak/build-dir \
  .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
```

The manifest enables network during the build so `go` can download modules. For a fully offline build, run `go mod vendor` at the repo root and change the `go build` line to use `-mod=vendor` (see manifest comments).

**Host freezes or reboots during the build** are often caused by **out-of-memory (OOM)** when OpenCV and Ninja compile many files at once. The manifest uses **`MAKEFLAGS`** (Leptonica/autotools) and **`NINJA_JOBS`** (Tesseract, OpenCV, `go build -p`). Lower both (e.g. `1`–`4`) if the machine struggles; raise them when you have RAM headroom. Also pass **`flatpak-builder --jobs=N`** to limit internal steps where supported. Note: some `flatpak-builder` versions ignore a top-level `jobs:` key in YAML—use `MAKEFLAGS` / `NINJA_JOBS` instead.

Install and run:

```bash
flatpak-builder --user --install --force-clean \
  .devcontainer/builds/linux/packaging/flatpak/build-dir \
  .devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml
flatpak run --user com.sqyre.app
```

The manifest is at `.devcontainer/builds/linux/packaging/flatpak/com.sqyre.app.yml`. It expects the app to be built from `./cmd/sqyre` and uses metadata from `com.sqyre.app.desktop` and `com.sqyre.app.appdata.xml` in the same directory.

---

## AppImage

### Prerequisites

- A **Linux build environment** with:
  - Go 1.24+
  - OpenCV and Tesseract (and Leptonica) dev libraries — same as in the main README “Linux (non-NixOS)” section (e.g. `libtesseract-dev`). The devcontainer installs OpenCV under **`/opt/opencv/linux/install`**; the AppImage recipe bundles `.so` files from there (or falls back to `/usr/local/lib`).
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
| **Flatpak**  | Repo root, manifest under `.devcontainer/builds/linux/packaging/flatpak/` | Manifest includes Leptonica, Tesseract, and OpenCV modules; first OpenCV build is slow |
| **AppImage** | Repo root, script in `appimage/`   | OpenCV + Tesseract on host + appimage-builder |

Both assume the application entrypoint is at **`./cmd/sqyre`** (see main README for building that target).
