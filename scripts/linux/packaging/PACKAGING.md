# Packaging Sqyre for Linux

Local and CI builds use **Rust** (`make` → `./bin/sqyre`; `make appimage` / `make flatpak`).

| Path | Contents |
|------|----------|
| `scripts/linux/packaging/appimage/` | AppImage recipe, build script, desktop file |
| `scripts/linux/packaging/flatpak/` | Flatpak manifest, AppStream metainfo, cargo-sources, build script |

App entrypoint: `sqyre-app` → binary name `sqyre`. App ID: `com.sqyre.app`.

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

## Flatpak (Flathub-ready)

Manifest: [`flatpak/com.sqyre.app.yml`](flatpak/com.sqyre.app.yml) — Freedesktop **25.08**, Rust SDK extension, leptonica + tesseract modules, `portal-capture` build. AppStream: [`com.sqyre.app.metainfo.xml`](flatpak/com.sqyre.app.metainfo.xml).

### Prerequisites

- `flatpak`, `flatpak-builder`, `ostree` (installed in the **devcontainer**)
- User namespaces / `bwrap` on the host (required by flatpak-builder)
- Flathub remote + runtimes (script installs them):
  - `org.freedesktop.Platform//25.08`, `Sdk//25.08`
  - `org.freedesktop.Sdk.Extension.rust-stable//25.08`
  - `org.freedesktop.Sdk.Extension.llvm21//25.08`

If native tools work but bubblewrap cannot create user namespaces (common in
Devcontainers), or tools are missing, `build-flatpak.sh` falls back to Docker
(`ghcr.io/flathub-infra/flatpak-github-actions:freedesktop-25.08`, `--privileged`,
host UID). Builds pass `--disable-rofiles-fuse` so they work without `/dev/fuse`.
If a prior Docker build left root-owned files under
`scripts/linux/packaging/flatpak/.flatpak-builder/`, the script reclaims them with
`sudo chown` before a native rebuild (otherwise flatpak-builder fails writing
`ccache.conf`).

### Build

```bash
make flatpak
# or: RELEASE_VERSION=1.2.3 scripts/linux/packaging/flatpak/build-flatpak.sh
```

Output: **`bin/com.sqyre.app.flatpak`**.

```bash
flatpak install --user bin/com.sqyre.app.flatpak
flatpak run com.sqyre.app
# Capability probe inside the sandbox:
flatpak run --command=sqyre-probe com.sqyre.app --json
```

### Offline crates (`cargo-sources.json`)

Flathub builds are offline. Keep [`cargo-sources.json`](flatpak/cargo-sources.json) in sync with `Cargo.lock`:

```bash
scripts/linux/packaging/flatpak/generate-cargo-sources.sh
```

Regenerate and commit whenever `Cargo.lock` changes.

### Sandbox / data / updates

- **No `--filesystem=home`** — Flatpak data is separate from AppImage/native `~/.sqyre`.
- **`--persist=.sqyre`** — required because Sqyre writes `$HOME/.sqyre` (not XDG). Maps to host `~/.var/app/com.sqyre.app/.sqyre`; without it Flatpak uses a tmpfs and every quit resets macros/settings/portal tokens.
- **Finish-args** (see comments in the YAML): X11 + Wayland, Pulse, DRI, `--device=input` (Wayland hotkeys), Notifications, host AT-SPI (`xdg-run/at-spi` + `org.a11y.Bus` for GNOME Wayland window list); ScreenCast/RemoteDesktop via portals.
- **Tesseract/Leptonica** install into `/app/lib` (`CMAKE_INSTALL_LIBDIR=lib` / `--libdir`) — `/app/lib64` is not on Flatpak’s runtime linker path.
- **Do not** ship a private `libpipewire` that shadows SPA plugins (same rule as AppImage).
- **In-app auto-update** is disabled under Flatpak (`FLATPAK_ID`); use `flatpak update`.

### Parity check

Treat launch as insufficient. After install on a graphical session, run `sqyre-probe` inside the Flatpak and confirm required caps (`capture.*`, `windows.list`, `input.open`, `hotkeys.start`, `outline.open`, `grab.open`) per [linux-desktop-parity](../../../.cursor/skills/linux-desktop-parity/SKILL.md). On GNOME Wayland, `windows.list` needs the host AT-SPI finish-args above.

---

## Bundled release (portable directory)

Self-contained **`bin/sqyre-bundle/`** with Tesseract/Leptonica `.so` files and `tessdata/eng.traineddata`. The binary uses `$ORIGIN/lib` rpath so it runs without a system `libtesseract`.

```bash
make release-bundle
./bin/sqyre-bundle/sqyre
```

**Heap-profile variant** (local leak hunts only — slower allocator, not for shipping):

```bash
make release-bundle-dhat
cd /tmp && SQYRE_MEM=1 /path/to/bin/sqyre-bundle-dhat/sqyre
# quit cleanly → dhat-heap.json in cwd; view at https://nnethercote.github.io/dh_view/dh_view.html
```

Uses a separate Cargo target dir (`target-dhat/`) so it does not overwrite `target/release/sqyre`.

Requires **patchelf** (installed in the devcontainer). Build on the target glibc family you intend to ship against (same constraint as AppImage). Does **not** run `make check` (unlike `make appimage` / `make flatpak`); run `make check` yourself before shipping.

**Wayland portal capture:** `libpipewire` / `libspa-*` are **not** bundled — the host PipeWire stack (GNOME/KDE already ship it) must provide SPA plugins. Bundling breaks with `can't make support.system handle`.

**Cue audio:** `libasound` is **not** bundled. Playback uses the PulseAudio protocol (PipeWire's pulse socket on current desktops). Bundled Ubuntu `libasound` looks for `libasound_module_pcm_pipewire.so` under `/usr/lib/x86_64-linux-gnu/alsa-lib/`, which Fedora does not have.

---

## Summary

| Format | Command | Main requirement |
|--------|---------|------------------|
| **Bundled dir** | `make release-bundle` | Rust + Tesseract dev libs + patchelf |
| **Bundled + dhat** | `make release-bundle-dhat` | Same; `dhat-heap` for allocation profiles (not shipping) |
| **AppImage** | `make appimage` | Rust + Tesseract on host + appimage-builder |
| **Flatpak** | `make flatpak` | flatpak-builder + Freedesktop 25.08 (+ Docker privileged fallback) |
