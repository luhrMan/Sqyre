<p align="center">
  <img src="crates/sqyre-app/assets/icons/sqyre.svg" width="120" height="120" alt="Sqyre logo" />
</p>

<h1 align="center">Sqyre</h1>

<p align="center">
  <strong>Desktop macro builder</strong> — automate mouse, keyboard, and screen-aware steps with a visual tree editor.
</p>

<p align="center">
  <a href="https://github.com/luhrMan/Squire/releases">Releases</a>
  ·
  <a href="docs/DEVELOPING.md">Developing</a>
  ·
  <a href="docs/RUST.md">Workspace</a>
  ·
  <a href="LICENSE">GPL-3.0</a>
</p>

---

## 📋 Project description

Sqyre is a desktop app for building and running macros **without writing code**. Each macro is a tree of actions: loops and branches for flow control, detection steps when the screen matters, and leaf actions for concrete mouse and keyboard input.

Macros, images, masks, and data tables live under **`~/.sqyre/`** (user home on every platform). The catalog of programs, reusable points, search areas, and templates is edited in-app; settings live in `settings.yaml` beside `db.yaml`.

**Platforms**

| Platform | Status |
|----------|--------|
| **Linux** | Shipped on **X11** and **Wayland** (portal ScreenCast / RemoteDesktop + EIS where the session supports them; Permissions settings in-app) |
| **Windows** | Released `.exe` (GDI capture, window focus, selection outline, hotkeys, tray; no MSI) |
| **macOS** | Capture/focus and releases not shipped yet |
| **WASM** | Browser editor zip for editing macros (no Run / capture / OCR) |

**Stack:** [egui](https://github.com/emilk/egui) · PureCV · Tesseract (`leptess`).

---

## ✨ Features

### Actions

| Category | Actions |
|----------|---------|
| **Mouse & keyboard** | Move, click, key, type |
| **Detection** | Image search (PureCV; multi-variant icons), OCR (Tesseract), find pixel — optional wait-until-found |
| **Variables** | Set (values + expressions), save to file or clipboard |
| **Control flow** | Loop, while, break/continue, for each row, if |
| **Miscellaneous** | Wait, pause, focus window, run macro, navigate select/key |

### Also in the app

- **Data editor** — programs, items (with icon variants), masks, points, search areas, collections, atlases; Tools tab for Overlay buttons, ScreenCap, and PixelCheck
- **Command palette** (`Ctrl+K`) — jump to macros, add actions, open editor tabs
- **Macro recording** — capture moves, clicks, keys, and waits, then review / copy into a tree
- **Hotkeys** — press or release, multiselect tag filter on the macro list (optional while-focused Program tags), chooser when multiple macros share a chord
- **System tray** — hide / show the main window
- Zip backups of the data directory; in-app auto-update from GitHub Releases (Linux/Windows)
- Global delay per macro; runtime variable panel while a macro runs

### Screenshots

Assets under `docs/images/` are generated from in-memory egui tests (`make docs-media`).

| | |
|---|---|
| Main window | ![Main window](docs/images/main-window.png) |
| Add action picker | ![Add action picker](docs/images/add-action-picker.png) |
| Data editor | ![Data editor](docs/images/data-editor.png) |
| Settings | ![Settings](docs/images/settings.png) |
| Command palette | ![Command palette](docs/images/command-palette.png) |

---

## 🚀 Quick start

**End users** — grab a build from [GitHub Releases](https://github.com/luhrMan/Squire/releases):

1. Download the Linux binary / AppImage, Windows `sqyre.exe`, or the WASM editor zip.
2. Run the binary (or open the WASM editor in a static file server).
3. Create a macro — the root is always a **loop** — add actions from the picker or command palette, then **Run** from the toolbar, a hotkey, or an overlay button.

**Developers** — open the repo in the **dev container** (Rust, Tesseract/Leptonica, and X11 link deps are preinstalled), then:

```bash
make            # → ./bin/sqyre (debug)
make run        # cargo run -p sqyre-app
```

> Build in the container; run the Linux binary on a host with a display. See [Developing](docs/DEVELOPING.md) for full setup.

---

## 📦 Installation

### From releases

| Artifact | Notes |
|----------|--------|
| Linux binary / AppImage | Native automation on X11 / Wayland |
| Windows `sqyre.exe` | Portable; no MSI installer |
| WASM editor zip | Edit macros in the browser; no Run / capture / OCR |

Shipped Linux/Windows builds can check GitHub Releases for updates (Ed25519-signed checksums). Local `0.0.0-dev` builds skip update checks.

### From source

**Recommended:** use the [dev container](.devcontainer/).

```bash
make                 # debug → ./bin/sqyre
make release         # release → ./bin/sqyre
make appimage        # → bin/Sqyre-*.AppImage
make windows         # → bin/sqyre.exe (Docker MinGW cross on Linux)
make wasm            # → bin/wasm/ (Trunk)
```

| Goal | Command |
|------|---------|
| Linux binary (default) | `make` / `make sqyre` → `./bin/sqyre` |
| Run without installing | `make run` |
| Release binary | `make release` |
| Windows / macOS | `make windows` · `make macos` (macOS host only) |
| AppImage | `make appimage` |
| WASM editor | `make wasm` → `bin/wasm/` |
| Tesseract data (dev fallback) | `make tessdata` |

Override cargo args with `CARGO_FLAGS=...`. Manual host setup (Rust ≥ 1.92, clang, Tesseract/Leptonica, X11 libs) is documented in [docs/DEVELOPING.md](docs/DEVELOPING.md) and [docs/RUST.md](docs/RUST.md).

---

## ⚙️ Configuration

### Data directory

| Path | Role |
|------|------|
| `~/.sqyre/` | Default data dir (all platforms; under the user home) |
| `db.yaml` | Macro database |
| `settings.yaml` | User preferences |
| `~/.config/sqyre/data_dir` | Optional pointer if you relocate the data directory (Linux/macOS) |
| `%APPDATA%\sqyre\tessdata\` | Windows auto-download location for OCR trained data |

Relocate the data dir from **Settings** (or set `sqyre_dir` in `settings.yaml`). The XDG pointer keeps the next launch pointed at the new location.

### Settings highlights

Most options are edited under **Settings** in the app. Notable areas:

| Area | Purpose |
|------|---------|
| Appearance | UI scale, font size, action colors, compact program headers |
| Recording | Hide app while recording |
| Safety | Release held inputs when a macro ends; While iteration budget; nested Run Macro depth |
| Backups | Scheduled zip backups + retention |
| Updates | Check GitHub Releases on startup |
| Hotkeys | Tag filters; optional “tags while focused Program” |
| Permissions (Linux) | Portal ScreenCast / input grants on Wayland |

### Environment variables

| Variable | Purpose |
|----------|---------|
| `SQYRE_TESSDATA` | Directory containing Tesseract `eng.traineddata` |
| `SQYRE_DIAG=1` | Write diagnostic timeline to `diag.log` under the data dir |
| `RELEASE_VERSION` / `VERSION` file | Stamp release builds for auto-update (maintainers) |

OCR discovers tessdata in this order: `SQYRE_TESSDATA`, system paths (and beside `sqyre.exe` on Windows), workspace `assets/tessdata` (dev), then auto-download into the user data path.

---

## 💻 Usage examples

### Build a macro

1. Launch `./bin/sqyre` (or your release binary).
2. Create a macro — the root is always a **loop**.
3. Add child actions from the picker or **Ctrl+K**.
4. Configure each node in its pinned in-tree editor (coordinates, keys, templates, OCR regions, variables), picking reusable points, search areas, and images from entity pickers.
5. Run from the toolbar, a **hotkey**, an **overlay button**, or after **recording** a sequence.

### Control flow

- **Image search**, **OCR**, **find pixel**, and **if** run child steps only when their condition matches.
- **Loop** / **while** / **for each row** repeat children; **break** / **continue** control those loops.
- **Esc** stops a running macro; **Esc+Ctrl+Alt+Shift** is the failsafe exit.

### Typical workflows

```text
# Image-gated click
Image Search (template) → Click (matched point)

# OCR branch
OCR (region, pattern) → If match → Type / Key

# Data-driven loop
For each row (table) → Move / Click using row variables
```

### Browser editor

```bash
make wasm
# serve bin/wasm/ with any static file server
```

Import/export `db.yaml` in the browser. Automation (Run, capture, OCR) requires a native build.

---

## 🧪 Running tests

```bash
make test                 # cargo nextest (falls back to cargo test)
make check                # fmt --check + clippy (-D warnings) + cargo deny
make smoke                # debug bin/sqyre --version (no display)
make coverage             # llvm-cov HTML + lcov under target/coverage/
make coverage-floors      # line-% gates for pure crates
make docs-media           # regenerate docs/images/ screenshots
make bench                # criterion (local only; not CI)
```

Headless CI uses null backends / stub hotkeys where OS hooks are unavailable. Coverage tooling (`cargo-llvm-cov`) is preinstalled in the dev container. Details: [docs/DEVELOPING.md](docs/DEVELOPING.md).

---

## Security

Sqyre drives mouse, keyboard, and screen capture on your machine. Treat macros and shared `db.yaml` / backup zips as **trusted local automation**.

**Built-in safeguards**

- **Failsafe** — Esc stops a run; Esc+Ctrl+Alt+Shift exits
- **Held-input release** — optionally release keys/buttons still held when a macro ends
- **Loop budgets** — configurable While iteration cap and nested Run Macro depth
- **Path confinement** — file save / load actions resolve under the data directory (no arbitrary path escape via `..` / symlinks)
- **Updates** — release checksums are Ed25519-signed; the client verifies `SHA256SUMS.sig` before trusting hashes (fail-closed if the public key is unconfigured)
- **Linux portals** — Wayland capture/input use desktop portals; grant or revoke from **Settings → Permissions**

**Practical advice**

- Only import macros and backups from sources you trust
- Review detection and type/key actions before running someone else’s tree
- Prefer the Permissions panel on Wayland over blanket compositor workarounds
- Keep auto-update enabled on release builds so you receive signed updates

Report security-sensitive issues privately to the maintainer via [GitHub](https://github.com/luhrMan/Squire) when possible.

---

## 📝 Contributing

Contributions are welcome. A practical path:

1. Open the repo in the **dev container** (or match the toolchain in `rust-toolchain.toml`).
2. Make focused changes; follow existing crate boundaries (`docs/RUST.md`).
3. Run `make check` and `make test` before opening a PR.
4. Prefer clean breaking changes over compatibility shims (project policy).
5. For platform code, use `#[cfg(target_os = "...")]` modules — not `cfg!()` with platform-only types.

Helpful docs:

- [Image Search](docs/IMAGE_SEARCH.md) — collections, variants, cell occupation
- [Developing](docs/DEVELOPING.md) — build, CI, packaging, coverage
- [Rust workspace](docs/RUST.md) — crate map
- [Documentation index](docs/README.md)

Optional local pre-push hooks: install [lefthook](https://lefthook.dev), then `lefthook install` (runs `make check-fmt` + `make clippy`).

---

## 📄 License

Sqyre is licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE).

---

## Donations

If Sqyre saves you time, consider supporting development:

- **Monero:** `85rMS89cS9M8w8cD7ByC1EVXqenx9VBooakM46MLFptN8aRr3uojqfFPUNapWjTk3DPKZy5hadwN6UoGYrt5c7qkTqVWKdU`
- **[GitHub Sponsors — @luhrMan](https://github.com/sponsors/luhrMan)**
