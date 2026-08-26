# Packaging Sqyre for Linux

Local and CI builds use **Rust** (`make` → `./bin/sqyre`; `make appimage` for AppImage).

| Path | Contents |
|------|----------|
| `scripts/linux/packaging/appimage/` | AppImage recipe, build script, desktop file |

App entrypoint: `sqyre-app` → binary name `sqyre`.

---

## AppImage

### Prerequisites

- Rust ≥ 1.92, clang, Tesseract/Leptonica, X11 link deps (same as [DEVELOPING.md](../../../docs/DEVELOPING.md))
- [appimage-builder](https://appimage-builder.readthedocs.io/en/stable/intro/install.html), plus `patchelf` and `squashfs-tools`

The **dev container** installs these. PureCV needs **no OpenCV**.

### Build

From the repo root:

```bash
make appimage
# or: RELEASE_VERSION=1.2.3 scripts/linux/packaging/appimage/build-appimage.sh
```

If `appimage-builder` / `mksquashfs` / `patchelf` are not on the host, the script **falls back to Docker** using [`.devcontainer/Dockerfile`](../../../.devcontainer/Dockerfile) (same image CI uses). Force a native-only attempt with `SQYRE_APPIMAGE_FORCE_NATIVE=1`.

Version resolution order: `RELEASE_VERSION` env → `VERSION` file → `crates/sqyre-app/Cargo.toml`.

Output: **`bin/*.AppImage`**. `sqyre.AppDir` and build artifacts stay under `scripts/linux/packaging/appimage/`.

### Tesseract data

The recipe copies `eng.traineddata` from `assets/tessdata/` or host `/usr/share/tessdata/` when present, and sets `TESSDATA_PREFIX` / `SQYRE_TESSDATA` at runtime.

---

## Bundled release (portable directory)

Self-contained **`bin/sqyre-bundle/`** with Tesseract/Leptonica `.so` files and `tessdata/eng.traineddata`. The binary uses `$ORIGIN/lib` rpath so it runs without a system `libtesseract`.

```bash
make release-bundle
./bin/sqyre-bundle/sqyre
```

Requires **patchelf** (installed in the devcontainer). Build on the target glibc family you intend to ship against (same constraint as AppImage). Does **not** run `make check` (unlike `make appimage`); run `make check` yourself before shipping.

**Wayland portal capture:** `libpipewire` / `libspa-*` are **not** bundled — the host PipeWire stack (GNOME/KDE already ship it) must provide SPA plugins. Bundling breaks with `can't make support.system handle`.

**Cue audio:** `libasound` is **not** bundled. Playback uses the PulseAudio protocol (PipeWire's pulse socket on current desktops). Bundled Ubuntu `libasound` looks for `libasound_module_pcm_pipewire.so` under `/usr/lib/x86_64-linux-gnu/alsa-lib/`, which Fedora does not have.

---

## Summary

| Format | Command | Main requirement |
|--------|---------|------------------|
| **Bundled dir** | `make release-bundle` | Rust + Tesseract dev libs + patchelf |
| **AppImage** | `make appimage` | Rust + Tesseract on host + appimage-builder |
