<p align="center">
  <img src="internal/assets/icons/sqyre.svg" width="120" height="120" alt="Sqyre logo" />
</p>

<h1 align="center">Sqyre</h1>

<p align="center">
  <strong>Desktop macro builder</strong> — automate mouse, keyboard, and screen-aware steps with a visual tree editor.
</p>

---

## What it does

Sqyre lets you build and run macros without writing code. Each macro is a tree of actions: loops and branches for flow control, detection steps when the screen matters, and leaf actions for concrete input. Macros, images, masks, and data tables live under **`~/.sqyre/`** (config in `db.yaml`).

**Platforms:** Linux and Windows (see [Developing](docs/DEVELOPING.md)).

---

## Actions

| Category | Actions |
|----------|---------|
| **Mouse & keyboard** | Move, click, key, type |
| **Detection** | Image search (OpenCV), OCR (Tesseract), find pixel |
| **Variables** | Set (values + expressions), for each row, save to file or clipboard |
| **Loop flow** | Loop, while, break, continue |
| **Miscellaneous** | Wait, pause, focus window, run macro, if (conditional), navigate select/key |

**Also in the app:** data editor for reusable images, masks, and tabular sources; macro hotkeys (on press or release); global delay per macro; runtime variable panel while a macro runs.

**Stack (Rust daily driver):** [egui](https://github.com/emilk/egui) · PureCV · Tesseract (`leptess`). Legacy Go/Fyne path: `make go` (see [migration](rust/MIGRATION.md)).

---

## Usage

1. **Build** — `make` (Rust) or `make windows` for the legacy Go Windows exe (see [Developing](docs/DEVELOPING.md)).
2. **Launch** `./bin/sqyre` (Linux Rust) — or `make run`.
3. **Create a macro** — the root is always a **loop**; add child actions from the picker.
4. **Configure** each node in its pinned in-tree tooltip editor (coordinates, keys, templates, OCR regions, variables, etc.), picking reusable points, search areas, and images from entity pickers.
5. **Run** from the toolbar, or assign a **hotkey** to the macro.

Branching actions (**image search**, **OCR**, **find pixel**, **if**) run child steps only when their condition matches. **Loop** / **while** / **for each row** repeat children; **break** and **continue** control those loops.

---

## Screenshots

Assets under `docs/images/` are generated from UI tests (`./scripts/generate-docs-media.sh`). CI checks they stay in sync.

| | |
|---|---|
| Main window | ![Main window](docs/images/main-window.png) |
| Add action picker | ![Add action picker](docs/images/add-action-picker.png) |
| Data editor | ![Data editor](docs/images/data-editor.png) |
| Building a macro | ![Demo](docs/images/demo-macro.gif) |

---

## Build (quick start)

**Recommended:** open the repo in the **dev container** — dependencies and OpenCV match what the app expects.

| Goal | Command |
|------|---------|
| Linux binary (Rust, default) | `make` / `make sqyre` → `./bin/sqyre` |
| Run without installing | `make run` |
| Legacy Go/Fyne binary | `make go` → `./bin/sqyre-go` |
| Windows exe (Go) | `make windows` → `bin/windows-amd64/` |
| AppImage (Go) | `make appimage` |
| Tesseract data | `make tessdata` |

Override Rust with `CARGO_FLAGS=...`. Go tags: `BUILD_TAGS=...` (default: `gocv_specific_modules`).

More detail — Rust workspace, host setup, Go tests, packaging — is in **[docs/DEVELOPING.md](docs/DEVELOPING.md)**, **[rust/README.md](rust/README.md)**, and **[docs/README.md](docs/README.md)**.

---

## License

Sqyre is licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE).

---

## Donations

If Sqyre saves you time, consider supporting development:

- **Monero:** `85rMS89cS9M8w8cD7ByC1EVXqenx9VBooakM46MLFptN8aRr3uojqfFPUNapWjTk3DPKZy5hadwN6UoGYrt5c7qkTqVWKdU`
- **[GitHub Sponsors — @luhrMan](https://github.com/sponsors/luhrMan)**
