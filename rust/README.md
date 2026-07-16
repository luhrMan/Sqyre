# Sqyre Rust workspace

In-repo Sqyre implementation (egui + PureCV). **`make` / `./bin/sqyre` is the only shipped binary** (Linux).

**Migration tracker:** [MIGRATION.md](./MIGRATION.md) (historical checklist — Go runtime removed).

## Crates

| Crate | Role |
|-------|------|
| `sqyre-varref` | `${name}` / `{name}` grammar |
| `sqyre-domain` | Macro + 21 action kinds |
| `sqyre-serialize` | YAML codecs |
| `sqyre-validate` | Names / action save checks |
| `sqyre-persist` | `~/.sqyre/db.yaml` + program catalog |
| `sqyre-executor` | Injected automation / capture / match / coords |
| `sqyre-match` | `TM_CCOEFF_NORMED` + mask + peak/dedup |
| `sqyre-vision` | RGB load, match façade, find-pixel, OCR preprocess / Tesseract |
| `sqyre-input` | `AutomationBackend` (rustautogui lite + arboard) |
| `sqyre-capture` | `ScreenCapturer` (Linux X11 absolute rects) |
| `sqyre-hotkeys` | Esc stop / failsafe (`hooks` feature; stub default) |
| `sqyre-app` | egui shell; Run/Stop macros |

## Develop

Requires **Rust ≥ 1.92** (egui 0.34 / PureCV). The `.devcontainer` pins `1.92.0` plus clang/Tesseract for OCR.

Linux automation/capture need X11 (`libx11-dev`, `libxtst-dev`).

From the repo root:

```bash
make                 # ./bin/sqyre (debug)
make rust-release    # ./bin/sqyre (release)
make rust-test
make run             # cargo run -p sqyre-app; loads ~/.sqyre/db.yaml
make appimage        # Linux AppImage
```

Or from `rust/`:

```bash
cargo test
cargo run -p sqyre-app
```

Do not expect X11 inside the container — build there, run the binary on the host.

Host binary: `./bin/sqyre` after `make`, or `./rust/target/debug/sqyre` from cargo. Esc stops a running macro; Esc+Ctrl+Shift exits (failsafe).

Still improving: Wayland, Windows/macOS automation + capture. CI releases Linux only.

OCR uses Tesseract (`leptess`). Override tessdata with `SQYRE_TESSDATA` if needed (dev fallback: `assets/tessdata`).
