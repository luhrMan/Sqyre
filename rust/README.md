# Sqyre Rust workspace

In-repo rewrite of Sqyre (egui + PureCV). Go under the repo root remains the daily driver until parity.

**Migration tracker:** [MIGRATION.md](./MIGRATION.md) (shared checklist — update status when landing work).

## Crates

| Crate | Role |
|-------|------|
| `sqyre-varref` | `${name}` / `{name}` grammar |
| `sqyre-domain` | Macro + 21 action kinds |
| `sqyre-serialize` | YAML codecs (Go `ActionToMap` / `DecodeMacroFromMap`) |
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

Requires **Rust ≥ 1.92** (egui 0.34 / PureCV). The slim `.devcontainer` pins `1.92.0` plus clang/Tesseract for OCR (no OpenCV/Go).

Linux automation/capture need X11 (`libx11-dev`, `libxtst-dev`).

From the repo root:

```bash
make rust            # ./bin/sqyre-rust (debug)
make rust-release    # ./bin/sqyre-rust (release)
make rust-test
make rust-run        # cargo run -p sqyre-app; loads ~/.sqyre/db.yaml
```

Or from `rust/`:

```bash
cargo test
cargo run -p sqyre-app
```

Do not expect X11 inside the container — build there, run the binary on the host.

Host binary: `./bin/sqyre-rust` after `make rust`, or `./rust/target/debug/sqyre` from cargo. Esc stops a running macro; Esc+Ctrl+Shift exits (failsafe).

Still improving: variables panel, data-editor polish, non-Linux platforms.

OCR uses Tesseract (`leptess`). Override tessdata with `SQYRE_TESSDATA` if needed.
