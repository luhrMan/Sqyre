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
| domain models (+ While, NavigateSelect) | ✅ (+Rust-ahead) | cutover pending |
| serialize (YAML codecs) | ✅ | cutover pending |
| validate | 🟡 | needs work |
| persist / config / settings | 🟡 | needs work |
| match (PureCV) | ✅ | cutover pending |
| vision / OCR | ✅ | cutover pending |
| executor (minus NavigateSelect) | 🟡 | needs work |
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
- [x] 19 Go action kinds in Rust — ✅ cutover pending
- [x] Rust-ahead: `While`, `NavigateSelect` (model/UI/serialize) — ✅ in Rust; **absent from Go** (intentional; no Go to delete)
- [x] Known-variable set / collect (decls, bindings, ImageSearch + ForEachRow builtins) — ✅ cutover pending — `domain/variables.rs`
- [ ] Builtin/runtime variable resolve parity vs `internal/macro` (monitor builtins, edge cases) — 🟡 needs work

### Serialize
- [x] Action + macro YAML codecs — ✅ cutover pending — `sqyre-serialize` ↔ `internal/models/serialize`
  - Notes: loads same `~/.sqyre/db.yaml`; includes `while` / `navigateselect`.

### Validate
- [x] Entity / variable names, search-area bounds, item grid — ✅ cutover pending — `sqyre-validate` ↔ `internal/validation`
- [ ] Full `ValidateAction` parity (only Key / Calculate / SetVariable / Pause today + any Go extras) — 🟡 needs work — file notes “subset of Go”

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
- [x] Mouse/keyboard/type/wait/pause/set/calc/save var / focus / image / pixel / OCR — ✅ cutover pending (Linux)
- [ ] `NavigateSelect` execute — ❌ stub — `executor/run.rs` “not implemented yet”
- [ ] Cross-check delay/retry/highlight/log parity vs Go executor_* — 🟡 needs work

### Input
- [x] Linux automation (rustautogui + clipboard) — ✅ cutover pending — `sqyre-input` ↔ `services/automation.go` / robotgo
- [ ] Windows / mac automation — ❌ needs work

### Capture / focus
- [x] Linux X11 capture + focus — ✅ cutover pending — `sqyre-capture` ↔ `internal/capture` + `screen` + window_*
- [ ] Non-Linux capturer / focuser — ❌ (`NullCapturer` / focus error) needs work
- [ ] Wayland capture/overlays — ❌ / 🟡 (Go already limited) needs work

### Hotkeys
- [x] Esc stop + Esc+Ctrl+Shift failsafe — ✅ cutover pending — `sqyre-hotkeys` (`hooks`) ↔ `internal/macrohotkey`
- [x] Pause continue-wait bridge — ✅ cutover pending ↔ `continue_wait` / pause state
- [x] Press-latch helpers (+ chord release wait) — ✅ cutover pending ↔ `internal/hotkeytrigger`
- [x] Screen-click bridge (point + search-area) — ✅ cutover pending — `hotkeys/screen_click.rs` ↔ recording / point pick
- [x] Per-macro hotkey register + chord fire to launch macros — ✅ cutover pending — `macro_hotkeys.rs` + app `HotkeyRecordUi`; suspend during pause/record
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
| Action tooltips view/edit | `ui/macro/action_tooltip_*` | `action_tooltip/` + var pills | 🟡 | needs work |
| Var pills / VarEntry | `ui/custom_widgets` | `var_pills.rs` | 🟡 | needs work |
| Entity pickers / recording overlays | `ui` pickers, `ui/recording` | `pickers`, capture overlays, screen-click, `hotkey_record` | 🟡 | needs work |
| Preview tooltips | custom_widgets / action_preview | `preview_tooltip.rs` | 🟡 | needs work |
| Data editor (programs/items/masks/collections/coords + variants) | `ui/editor` | `data_editor.rs` + `icon_variants.rs` | 🟡 | needs work |
| Settings panel (prefs, paths, fonts, colors) | `ui/settings.go` | `settings.rs` + `persist/settings.rs` | 🟡 | needs work |
| Action logs UI | macro log popup | `action_logs_ui.rs` | 🟡 | needs work |
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
- [ ] NavigateSelect either implemented or deliberately removed from domain/serialize/UI
- [ ] ValidateAction parity (or Go checks explicitly dropped)
- [ ] Real-user smoke: load existing `db.yaml`, run macros, Esc/failsafe
- [ ] Then delete Go packages/files that Rust owns; mark 🔪 here

---

## Top remaining priorities

1. **UI daily-driver polish** — data editor, settings restart, variables panel, add-action UX
2. **NavigateSelect executor** — stub or remove from product surface
3. **ValidateAction full parity**
4. **Cutover** — switch default binary; only then 🔪 Go

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
