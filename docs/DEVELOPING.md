# Developing Sqyre

## Dev container (recommended)

Open the repository in the dev container (`.devcontainer/`). It includes Rust 1.92, clang, Tesseract/Leptonica, X11 link deps, and AppImage packaging tools (`appimage-builder`, squashfs-tools).

From the repo root:

```bash
make            # ./bin/sqyre (debug)
make release    # ./bin/sqyre (release)
make run        # cargo run -p sqyre-app
make check      # fmt --check + clippy (-D warnings) + cargo deny
make machete    # unused crate deps
make test       # cargo nextest (falls back to cargo test)
make coverage   # llvm-cov HTML + lcov under target/coverage/
make docs-media # regenerate docs/images screenshots
make appimage   # bin/*.AppImage (Linux)
make tessdata   # download eng.traineddata into assets/tessdata/
```

Run `make help` for the full target list. Workspace layout: [RUST.md](./RUST.md).

Build caches (all gitignored):

| Path | Role |
|------|------|
| `target/` | Incremental compile artifacts (host + docker bind-mount) |
| `.cargo-home/` | Optional workspace-local cargo/rustup install |
| `.cache/cargo/` | Cargo registry/git cache used by CI and docker AppImage builds |
| Dev container volume `sqyre-cargo-home` | Persistent `/home/vscode/.cargo` in the container |

`make appimage` via Docker reuses `CARGO_HOME` when Make exports `.cargo-home`, otherwise `.cache/cargo`.

---

## Make targets

| Target | Output |
|--------|--------|
| `all` / `sqyre` | `bin/sqyre` (debug) — **default** |
| `release` | `bin/sqyre` (release) |
| `check-fmt` | `cargo fmt --all -- --check` |
| `fmt` | `cargo fmt --all` (write) |
| `clippy` | `cargo clippy --workspace --all-targets` (`-D warnings`) |
| `deny` | `cargo deny check` (licenses / advisories / bans / sources) |
| `machete` | `cargo machete` (unused dependencies) |
| `check` | `check-fmt` + `clippy` + `deny` (CI quality gates) |
| `test` | `cargo nextest run --workspace` (falls back to `cargo test`) |
| `coverage` | llvm-cov HTML + `lcov.info` under `target/coverage/` (no % gate) |
| `run` | `cargo run -p sqyre-app` |
| `docs-media` | Regenerate `docs/images/` screenshots |
| `appimage` | `bin/Sqyre-*.AppImage` |
| `tessdata` | Tesseract trained data via `scripts/download-tessdata.sh` |

Set `CARGO_FLAGS` for extra cargo args. Set `RELEASE_VERSION` (or write a `VERSION` file) before `make appimage` to stamp the AppImage name.

CI builds and releases **Linux** binaries and AppImages only. Windows/macOS automation is not shipped yet.

---

## Native dependencies

| Resource | Purpose |
|----------|---------|
| [.devcontainer/Dockerfile](../.devcontainer/Dockerfile) | Rust + Tesseract + AppImage tools |
| [crates/sqyre-app/assets/icons/](../crates/sqyre-app/assets/icons/) | Brand icons (embedded SVG) |
| [assets/tessdata/](../assets/tessdata/) | Optional local `eng.traineddata` fallback |

OCR uses system tessdata when available, or `SQYRE_TESSDATA` / `assets/tessdata` when developing.

---

## Manual setup (without dev container)

Prefer the container when possible. Needs **Rust ≥ 1.92**, clang, Tesseract/Leptonica, and X11 libs (`libx11-dev`, `libxtst-dev`, …). See [RUST.md](./RUST.md).

```bash
make            # or: cargo build -p sqyre-app
./bin/sqyre
```

For AppImage on the host, also install `appimage-builder`, `patchelf`, and `squashfs-tools`.

---

## Tests

```bash
make test
# or: cargo test
```

Headless CI uses Null* backends / stub hotkeys where hooks are unavailable.

### README screenshots

In-memory egui goldens live under `docs/images/` (test: `cargo test -p sqyre-app --test docs_screenshots`).

```bash
make docs-media
# or: SQYRE_UPDATE_SCREENSHOTS=1 ./scripts/generate-docs-media.sh
```

Needs wgpu (lavapipe in the dev container / CI image).

---

## Packaging

See [scripts/linux/packaging/PACKAGING.md](../scripts/linux/packaging/PACKAGING.md) for AppImage builds.
