# Sqyre Rust workspace

Cargo workspace at the repo root (egui + PureCV). Shipped CI releases: Linux binary + AppImage, Windows `sqyre.exe` (MinGW cross), and the WASM editor zip. macOS is compile-check only for now.

## Layout

| Path | Role |
|------|------|
| `Cargo.toml` | Workspace manifest |
| `crates/*` | Library + app crates |
| `crates/sqyre-app/assets/icons/` | Brand icons (embedded by the app) |
| `assets/tessdata/` | Optional local OCR trained data |

## Crates

| Crate | Role |
|-------|------|
| `sqyre-varref` | `${name}` / `{name}` grammar |
| `sqyre-domain` | Macro + 21 action kinds |
| `sqyre-ui-model` | Display params / tree pills / action colors + glyphs (app-facing UI chrome) |
| `sqyre-serialize` | YAML codecs |
| `sqyre-validate` | Names / action save checks |
| `sqyre-persist` | `~/.sqyre/db.yaml` + program catalog |
| `sqyre-executor` | Injected automation / capture / match / coords |
| `sqyre-match` | All six OpenCV `TM_*` methods + mask + peak/dedup |
| `sqyre-vision` | RGB load, match façade, find-pixel, OCR preprocess / Tesseract |
| `sqyre-input` | `AutomationBackend` (rustautogui lite + arboard) |
| `sqyre-capture` | `ScreenCapturer` (`OsCapturer`: Linux X11, Windows GDI, macOS stub) |
| `sqyre-hotkeys` | Esc stop / failsafe (`hooks` feature; stub default) |
| `sqyre-app` | egui shell; Run/Stop macros |

## Develop

Requires **Rust ≥ 1.92** (egui 0.34 / PureCV). The repo pins `1.92.0` via [`rust-toolchain.toml`](../rust-toolchain.toml); the `.devcontainer` matches that plus clang/Tesseract for OCR.

Linux automation/capture need X11 (`libx11-dev`, `libxtst-dev`). Windows capture uses GDI (`windows` crate); macOS capture is still stubbed.

From the repo root:

```bash
make                 # ./bin/sqyre (debug)
make release         # fmt + check, then ./bin/sqyre (release)
make windows         # fmt + check, then ./bin/sqyre.exe (Docker MinGW cross / native on Windows)
make macos           # fmt + check, then ./bin/sqyre (macOS host)
make check           # fmt --check + clippy (-D warnings) + cargo deny
make test            # cargo nextest (falls back to cargo test)
make coverage        # llvm-cov HTML + lcov under target/coverage/
make run             # cargo run -p sqyre-app; loads ~/.sqyre/db.yaml
make appimage        # fmt + check, then Linux AppImage
make wasm            # fmt + check, then bin/wasm/ GUI-only browser editor
```

Or directly:

```bash
cargo nextest run --workspace
cargo run -p sqyre-app
```

Optional host tools (also installed in the `.devcontainer` image): `cargo-nextest`, `cargo-deny`, `cargo-llvm-cov`, `cargo-machete`.

Optional local pre-push hooks: install [lefthook](https://lefthook.dev), then `lefthook install` (runs `make check-fmt` + `make clippy`).

Do not expect X11 inside the container — build there, run the binary on the host.

Host binary: `./bin/sqyre` after `make`, or `./target/debug/sqyre` from cargo. Esc stops a running macro; Esc+Ctrl+Shift exits (failsafe).

Still improving: Wayland, macOS capture, macOS window focus, macOS releases. CI also `cargo check`s macOS on PRs.

OCR uses Tesseract (`leptess`). Override tessdata with `SQYRE_TESSDATA` if needed (dev fallback: `assets/tessdata`).
