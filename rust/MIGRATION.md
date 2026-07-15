# Go → Rust migration checklist

Shared agent tracker. Go at repo root remains the **daily driver** until cutover. Prefer clean breaking changes (no dual-path shims).

Update boxes/status when you land or delete work. Keep notes short.

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Feature-complete in Rust (library or UI) |
| 🟡 | Partial / WIP / Linux-only / subset vs Go |
| ❌ | Not started or stubbed |
| 🔪 | Go removed after Rust owned the path end-to-end |

**Delete rule:** Only remove Go when Rust owns the path **and** nothing still boots/runs via Go for that code. Until default binary is Rust, mark library parity as **cutover pending** — do not delete live Go.

| Disposition | Meaning |
|-------------|---------|
| `safe to delete Go` | Rust owns it; Go unused — delete now |
| `cutover pending` | Rust parity OK; Go still daily-driver / still imported |
| `needs work` | Gaps before parity or cutover |

**Deleted this pass:** nothing safe to delete yet.

---

## Status at a glance

| Area | Status | Disposition |
|------|--------|-------------|
| varref | ✅ | cutover pending |
| domain models (+ While, NavigateSelect, NavigateKey) | ✅ (+Rust-ahead) | cutover pending |
| serialize (YAML codecs) | ✅ | cutover pending |
| validate | ✅ | cutover pending |
| persist / config / settings | 🟡 | needs work |
| match (PureCV) | ✅ | cutover pending |
| vision / OCR | ✅ | cutover pending |
| executor | 🟡 | needs work |
| input (automation) | 🟡 Linux | needs work |
| capture / window focus | 🟡 Linux X11 | needs work |
| hotkeys (Esc / failsafe / pause / screen-click / macro) | ✅ Linux | cutover pending |
| UI / egui app | 🟡 | needs work |
| Win / mac / Wayland | ❌ / 🟡 | needs work |
| Default binary cutover | ❌ | needs work |

---

## Libraries

### Variable grammar
- [x] `sqyre-varref` ↔ `internal/varref` — ✅ cutover pending
  - Notes: grammar/`${}`/`{}` parity; Go still used by serialize/macro/UI.

### Domain models
- [x] Programs, macros, variables, coords, collections — ✅ cutover pending — `sqyre-domain` ↔ `internal/models` (+ `internal/macro` resolve helpers)
- [x] 19 Go action kinds in Rust (+ `Calculate` merged into `Set`) — ✅ cutover pending — 21 kinds total (`While` / `NavigateSelect` / `NavigateKey` Rust-ahead)
- [x] Expression eval + Set value resolve — ✅ cutover pending — `domain/expr.rs` + `domain/set_value.rs` (moved out of executor)
- [x] Rust-ahead: `While`, `NavigateSelect` (+ `NavigateKey` branches) — ✅ in Rust; **absent from Go** (intentional; no Go to delete)
- [x] Known-variable set / collect (decls, bindings, ImageSearch + ForEachRow builtins) — ✅ cutover pending — `domain/variables.rs`
- [ ] Builtin/runtime variable resolve parity vs `internal/macro` (monitor builtins, edge cases) — 🟡 needs work

### Serialize
- [x] Action + macro YAML codecs — ✅ cutover pending — `sqyre-serialize` ↔ `internal/models/serialize`
  - Notes: loads same `~/.sqyre/db.yaml`; includes `while` / `navigateselect` / `navigatekey`. Legacy `calculate` YAML decodes as `setvariable`. ImageSearch dropped unused `rowsplit`/`colsplit`.

### Validate
- [x] Entity / variable names, search-area bounds, item grid — ✅ cutover pending — `sqyre-validate` ↔ `internal/validation`
- [x] `ValidateAction` parity (bindings + Key / Set / Pause + expression structure) — ✅ cutover pending — `validate_action` + entry validators ↔ `ValidateAction` / `macro.Validate*`
  - Notes: `EntryValidation` / unknown-var warnings in crate; UI VarEntry live warnings still partial.

### Persist / config
- [x] `db.yaml` Database load/save — ✅ cutover pending — `sqyre-persist` ↔ `internal/models/repositories` + serialize
- [x] Program catalog CRUD (+ item rename moves icon/variant files) — ✅ cutover pending — `persist/programs.rs` ↔ program repos
- [x] Paths / `sqyre_dir` / `AutoPic` — ✅ cutover pending — `sqyre-persist` ↔ `internal/config`
- [x] User settings file load/save + color prefs — ✅ cutover pending — `persist/settings.rs` ↔ settings prefs
- [ ] Settings restart-after-change parity — ❌ / 🟡 — `services/restart_*` not ported

### Match / vision
- [x] Template match (CCOEFF_NORMED, mask, peaks/dedup) — ✅ cutover pending — `sqyre-match` ↔ gocv/`services` image search path
- [x] Find-pixel, match façade, OCR preprocess + Tesseract — ✅ cutover pending — `sqyre-vision` ↔ `internal/vision` + OCR
  - Notes: PureCV vs GoCV — behavioral re-prove on real macros; tessdata via `SQYRE_TESSDATA`.

### Executor / automation
- [x] Injected backends (`AutomationBackend`, capturer, matcher, focuser, OCR) — ✅ cutover pending — `sqyre-executor` ↔ `internal/services`
- [x] Flow: loop / while / break / continue / conditional / runmacro / foreach — ✅ cutover pending
- [x] Mouse/keyboard/type/wait/pause/set/save var / focus / image / pixel / OCR — ✅ cutover pending (Linux); Set evaluates `${refs}` and arithmetic expressions
- [x] `NavigateSelect` execute (+ `NavigateKey` chord branches) — ✅ `executor/navigate.rs` (grid via `CoordinateResolver::collection_grid`)
- [ ] Cross-check delay/retry/highlight/log parity vs Go executor_* — 🟡 needs work

### Input
- [x] Linux automation (rustautogui + clipboard) — ✅ cutover pending — `sqyre-input` ↔ `services/automation.go` / robotgo
- [ ] Windows / mac automation — ❌ needs work

### Capture / focus
- [x] Linux X11 capture + focus — ✅ cutover pending — `sqyre-capture` ↔ `internal/capture` + `screen` + window_*
- [x] X11 selection outline (recording HUD rects) — ✅ cutover pending — `SelectionOutline` + app `recording_overlay` ↔ `ui/recording`
- [ ] Non-Linux capturer / focuser / outline — ❌ (`NullCapturer` / stub outline / focus error) needs work
- [ ] Wayland capture/overlays — ❌ / 🟡 (Go already limited) needs work

### Hotkeys
- [x] Esc stop + Esc+Ctrl+Shift failsafe — ✅ cutover pending — `sqyre-hotkeys` (`hooks`) ↔ `internal/macrohotkey`
- [x] Continue-wait bridge (single chord + `wait_for_any_chord` / hold-repeat) — ✅ cutover pending — Pause + NavigateSelect ↔ `continue_wait`
- [x] Press-latch helpers (+ chord release wait) — ✅ cutover pending ↔ `internal/hotkeytrigger`
- [x] Screen-click bridge (point + search-area) — ✅ cutover pending — `hotkeys/screen_click.rs` ↔ recording / point pick
- [x] Per-macro hotkey register + chord fire to launch macros — ✅ cutover pending — `macro_hotkeys.rs` + app `HotkeyRecordUi`; suspend during pause/record/navigate wait
- [x] `nohook`/NullHotkeys CI story documented — ✅ (feature off = stub)

---

## App / UI (`sqyre-app` ↔ `ui/` + `cmd/sqyre` + `internal/app`)

Rust boots via `cargo run -p sqyre-app` → `./rust/target/debug/sqyre`. Go boots via `cmd/sqyre` → `internal/app.Run()` (Fyne). **Do not delete Go entrypoint** until cutover.

| Feature | Go | Rust | Status | Disposition |
|---------|----|------|--------|-------------|
| Shell / single-instance / tray | `internal/app`, `ui` tray | `main`, `single_instance`, `tray` | 🟡 | needs work |
| Macro list + tree + run/stop | `ui/macro` | `main`, tree_* | 🟡 | needs work |
| Macro name / delay / tags | `ui/macro` meta | `macro_meta.rs` | 🟡 | needs work |
| Tree DnD | `ui/macro` tree_dnd* | `tree_dnd.rs` | 🟡 | needs work |
| Tree undo/history | macro undo | `tree_history.rs` | 🟡 | needs work |
| Tree clipboard | `tree_clipboard.go` | `tree_clipboard.rs` cut/copy/paste | 🟡 | needs work |
| Action tooltips view/edit | `ui/macro/action_tooltip_*` | `action_tooltip/` (+ `sections`, NavigateSelect/Key editors) | 🟡 | needs work |
| Theme (dark + Sqyre gold) | `ui/theme.go` | `theme.rs` | 🟡 | needs work |
| Native file/folder dialogs | Fyne / OS pickers | `file_dialogs.rs` (rfd + Tokio enter) | 🟡 | needs work |
| Var pills / VarEntry | `ui/custom_widgets` | `var_pills.rs` (+ validate helpers) | 🟡 | needs work |
| Entity pickers / recording overlays | `ui` pickers, `ui/recording` | `pickers`, X11 `SelectionOutline` + `recording_overlay`, `hotkey_record` | 🟡 | needs work |
| Preview tooltips | custom_widgets / action_preview | `preview_tooltip.rs` | 🟡 | needs work |
| Data editor (programs/items/masks/collections/coords + variants) | `ui/editor` | `data_editor.rs` + `icon_variants.rs` | 🟡 | needs work |
| Settings panel (prefs, paths, fonts, colors) | `ui/settings.go` | `settings.rs` + `persist/settings.rs` + theme | 🟡 | needs work |
| Action logs UI | macro log popup | `action_logs_ui.rs` (incl. clear) | 🟡 | needs work |
| Variables panel + runtime vars | `macro_variables`, runtime_vars | domain/app (partial) | 🟡 | needs work |
| Add-action picker / colors / icons | `ui` + assets | assets + labels (partial) | 🟡 | needs work |
| Doc screenshot / golden pipeline | `ui/screenshot`, `testsupport` | — | ❌ | needs work (or drop) |
| Restart after settings | `services/restart_*` | — | ❌ | needs work |
| Default binary = Rust | `Makefile` / `cmd/sqyre` | `sqyre-app` | ❌ | cutover pending |

---

## Platform matrix

- [x] Linux X11 — 🟡 primary Rust target (automation + capture)
- [ ] Wayland — ❌ / 🟡 needs work
- [ ] Windows — ❌ needs work (Go has window/overlay paths)
- [ ] macOS — ❌ needs work
- [ ] Headless test story (Null* backends / no hooks) — 🟡 present; keep in sync with Go `nohook`

---

## Cutover gate (all must be true before deleting Go runtime)

- [ ] Rust is default `make` / shipped binary
- [x] Macro hotkey launch works
- [ ] Data editor + settings + recording pickers usable for daily macros
- [x] NavigateSelect implemented (with NavigateKey subaction branches)
- [x] ValidateAction parity (or Go checks explicitly dropped)
- [ ] Real-user smoke: load existing `db.yaml`, run macros, Esc/failsafe
- [ ] Then delete Go packages/files that Rust owns; mark 🔪 here

---

## Top remaining priorities

1. **UI daily-driver polish** — data editor, settings restart, variables panel, add-action UX
2. **Cutover** — switch default binary; only then 🔪 Go

---

## Map (quick)

| Go | Rust |
|----|------|
| `internal/varref` | `sqyre-varref` |
| `internal/models` (+ actions) | `sqyre-domain` |
| `internal/models/serialize` | `sqyre-serialize` |
| `internal/validation` | `sqyre-validate` |
| `internal/config` + repos | `sqyre-persist` |
| `internal/services` executor | `sqyre-executor` |
| image match | `sqyre-match` |
| `internal/vision` | `sqyre-vision` |
| automation / robotgo | `sqyre-input` |
| `internal/capture` + `screen` | `sqyre-capture` |
| `macrohotkey` / `hookkeys` / `hotkeytrigger` | `sqyre-hotkeys` |
| `ui/` + `cmd/sqyre` | `sqyre-app` |

See also: [README.md](./README.md), [docs/DEVELOPING.md](../docs/DEVELOPING.md).
