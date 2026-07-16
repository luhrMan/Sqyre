# Developing Sqyre

## Dev container (recommended)

Open the repository in the dev container (`.devcontainer/`). It includes Rust 1.92, clang, Tesseract/Leptonica, X11 link deps, and AppImage packaging tools (`appimage-builder`, squashfs-tools).

From the repo root:

```bash
make                # ./bin/sqyre (debug)
make rust-release   # ./bin/sqyre (release)
make run            # cargo run -p sqyre-app
make rust-test
make appimage       # bin/*.AppImage (Linux)
make tessdata       # download eng.traineddata into assets/tessdata/
```

Run `make help` for the full target list.

Migration notes (historical Go → Rust cutover): [rust/MIGRATION.md](../rust/MIGRATION.md).

---

## Make targets

| Target | Output |
|--------|--------|
| `all` / `sqyre` / `rust` | `bin/sqyre` (debug) — **default** |
| `rust-release` | `bin/sqyre` (release) |
| `rust-test` | `cargo test` in `rust/` |
| `run` / `rust-run` | `cargo run -p sqyre-app` |
| `appimage` | `bin/Sqyre-*.AppImage` |
| `tessdata` | Tesseract trained data via `scripts/download-tessdata.sh` |

Set `CARGO_FLAGS` for extra cargo args. Set `RELEASE_VERSION` (or write a `VERSION` file) before `make appimage` to stamp the AppImage name.

CI builds and releases **Linux** binaries and AppImages only. Windows/macOS automation is not shipped yet.

---

## Native dependencies

| Resource | Purpose |
|----------|---------|
| [.devcontainer/Dockerfile](../.devcontainer/Dockerfile) | Rust + Tesseract + AppImage tools |
| [assets/icons/](../assets/icons/) | Brand icons (embedded SVG) |
| [assets/tessdata/](../assets/tessdata/) | Optional local `eng.traineddata` fallback |

OCR uses system tessdata when available, or `SQYRE_TESSDATA` / `assets/tessdata` when developing.

---

## Manual setup (without dev container)

Prefer the container when possible. Needs **Rust ≥ 1.92**, clang, Tesseract/Leptonica, and X11 libs (`libx11-dev`, `libxtst-dev`, …). See [rust/README.md](../rust/README.md).

```bash
make            # or: cd rust && cargo build -p sqyre-app
./bin/sqyre
```

For AppImage on the host, also install `appimage-builder`, `patchelf`, and `squashfs-tools`.

---

## Tests

```bash
make rust-test
# or: cd rust && cargo test
```

Headless CI uses Null* backends / stub hotkeys where hooks are unavailable.

---

## Packaging

See [scripts/linux/packaging/PACKAGING.md](../scripts/linux/packaging/PACKAGING.md) for AppImage builds. Flatpak is not currently maintained (desktop/appdata stubs remain under `scripts/linux/packaging/flatpak/` for a future rewrite).
