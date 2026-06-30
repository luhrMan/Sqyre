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
| **Variables** | Set, calculate, for each row, save to file or clipboard |
| **Loop flow** | Loop, break, continue |
| **Miscellaneous** | Wait, pause, focus window, run macro, if (conditional) |

**Also in the app:** data editor for reusable images, masks, and tabular sources; macro hotkeys (on press or release); global delay per macro; runtime variable panel while a macro runs.

**Stack:** [Fyne](https://fyne.io/) · [robotgo](https://github.com/go-vgo/robotgo) · [gocv](https://gocv.io/) / OpenCV · [gosseract](https://github.com/otiai10/gosseract) / Tesseract

---

## Usage

1. **Build or install** for your OS — `make linux` or `make windows` (see [Developing](docs/DEVELOPING.md)).
2. **Launch** `./bin/sqyre` (Linux) or the Windows binary from `bin/windows-amd64/`.
3. **Create a macro** — the root is always a **loop**; add child actions from the picker.
4. **Configure** each node in its dialog (coordinates, keys, templates, OCR regions, variables, etc.).
5. **Run** from the toolbar, or assign a **hotkey** to the macro.

Branching actions (**image search**, **OCR**, **find pixel**, **if**) run child steps only when their condition matches. **Loop** / **for each row** repeat children; **break** and **continue** control those loops.

---

## Screenshots

Assets under `docs/images/` are generated from UI tests (`./scripts/generate-docs-media.sh`). CI checks they stay in sync.

| | |
|---|---|
| Main window | ![Main window](docs/images/main-window.png) |
| Add action picker | ![Add action picker](docs/images/add-action-picker.png) |
| Data editor | ![Data editor](docs/images/data-editor.png) |
| Building a macro | ![Demo](docs/images/demo-macro.gif) |

<details>
<summary>Action dialog screenshots</summary>

| Category | |
|----------|---|
| Mouse & keyboard | [Move](docs/images/action-dialog-move.png) · [Click](docs/images/action-dialog-click.png) · [Key](docs/images/action-dialog-key.png) · [Type](docs/images/action-dialog-type.png) |
| Detection | [Image search](docs/images/action-dialog-imagesearch.png) · [OCR](docs/images/action-dialog-ocr.png) · [Find pixel](docs/images/action-dialog-findpixel.png) |
| Variables | [Set](docs/images/action-dialog-setvariable.png) · [Calculate](docs/images/action-dialog-calculate.png) · [For each row](docs/images/action-dialog-foreachrow.png) · [Save to](docs/images/action-dialog-savevariable.png) |
| Miscellaneous | [Wait](docs/images/action-dialog-wait.png) · [Focus window](docs/images/action-dialog-focuswindow.png) · [Run macro](docs/images/action-dialog-runmacro.png) · [Loop](docs/images/action-dialog-loop.png) |

</details>

---

## Build (quick start)

**Recommended:** open the repo in the **dev container** — dependencies and OpenCV match what the app expects.

| Goal | Command |
|------|---------|
| Linux dev binary | `make linux` → `./bin/sqyre` |
| Windows exe | `make windows` → `bin/windows-amd64/` |
| AppImage | `make appimage` |
| Tesseract data | `make tessdata` |

Override Go build tags with `BUILD_TAGS=...` (default: `gocv_specific_modules`).

More detail — manual host setup, tests, profiling, packaging — is in **[docs/DEVELOPING.md](docs/DEVELOPING.md)** and **[docs/README.md](docs/README.md)**.

---

## License

Sqyre is licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE).

---

## Donations

If Sqyre saves you time, consider supporting development:

- **Monero:** `85rMS89cS9M8w8cD7ByC1EVXqenx9VBooakM46MLFptN8aRr3uojqfFPUNapWjTk3DPKZy5hadwN6UoGYrt5c7qkTqVWKdU`
- **[GitHub Sponsors — @luhrMan](https://github.com/sponsors/luhrMan)**
