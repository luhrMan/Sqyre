# Go → Rust migration checklist

Shared agent tracker. **Rust is the default daily driver** (`make` → `./bin/sqyre`). Go/Fyne remains available via `make go` → `./bin/sqyre-go` until 🔪 deletion. Prefer clean breaking changes (no dual-path shims).

Update boxes/status when you land or delete work. Keep notes short.

## Legend

| Mark | Meaning                                         |
| ---- | ----------------------------------------------- |
| ✅    | Feature-complete in Rust (library or UI)        |
| 🟡   | Partial / WIP / Linux-only / subset vs Go       |
| ❌    | Not started or stubbed                          |
| 🔪   | Go removed after Rust owned the path end-to-end |

**Delete rule:** Only remove Go when Rust owns the path **and** nothing still boots/runs via Go for that code. Default binary is now Rust; keep `cmd/sqyre` / `ui/` / packaging until smoke + UI polish + release pipelines switch.

| Disposition         | Meaning                                                |
| ------------------- | ------------------------------------------------------ |
| `safe to delete Go` | Rust owns it; Go unused — delete now                   |
| `cutover pending`   | Rust parity OK; Go still imported / packaging / tests  |
| `needs work`        | Gaps before parity or full Go deletion                 |

**Deleted this pass:** Go `Calculate` action type (merged into `Set`; legacy YAML still decodes). ImageSearch unused `rowsplit`/`colsplit` fields removed (legacy keys ignored on load). Nothing else 🔪 yet — packages stay until cutover gate clears.

**Shared DB hazard:** Rust and Go share `~/.sqyre/db.yaml`. Rust can persist `while` / `navigateselect` / `navigatekey` (and other Rust-ahead kinds) that Go cannot load. Do not round-trip the same DB through Go after editing those in Rust.

---

## Status at a glance

| Area                                                    | Status          | Disposition     |
| ------------------------------------------------------- | --------------- | --------------- |
| varref                                                  | ✅               | cutover pending |
| domain models (+ While, NavigateSelect, NavigateKey)    | ✅ (+Rust-ahead) | cutover pending |
| serialize (YAML codecs)                                 | ✅               | cutover pending |
| validate                                                | ✅               | cutover pending |
| persist / config / settings (library)                   | ✅               | cutover pending |
| match (PureCV)                                          | ✅               | cutover pending |
| vision / OCR                                            | ✅               | cutover pending |
| executor                                                | 🟡              | needs work      |
| input (automation)                                      | 🟡 Linux        | needs work      |
| capture / window focus                                  | 🟡 Linux X11    | needs work      |
| hotkeys (Esc / failsafe / pause / screen-click / macro) | ✅ Linux         | cutover pending |
| UI / egui app                                           | 🟡              | needs work      |
| Win / mac / Wayland                                     | ❌ / 🟡          | needs work      |
| Default binary = Rust (`make` / `./bin/sqyre`)          | ✅               | done (local)    |
| Shipped binary (CI / AppImage / Windows)                | ❌ still Go      | needs work      |

---

## Libraries

### Variable grammar

- [x] `sqyre-varref` ↔ `internal/varref` — ✅ cutover pending
  - Notes: grammar/`${}`/`{}` parity; Go still used by serialize/macro/UI.

### Domain models

- [x] Programs, macros, variables, coords, collections — ✅ cutover pending — `sqyre-domain` ↔ `internal/models` (+ `internal/macro` resolve helpers)
- [x] 18 Go action kinds in Rust (+ `Calculate` already merged into `Set`) — ✅ cutover pending — 21 kinds total (`While` / `NavigateSelect` / `NavigateKey` Rust-ahead). Go `calculate.go` deleted; legacy YAML → Set.
- [x] Expression eval + Set value resolve — ✅ cutover pending — `domain/expr.rs` + `domain/set_value.rs` (moved out of executor)
- [x] Rust-ahead: `While`, `NavigateSelect` (+ `NavigateKey` branches) — ✅ in Rust; **absent from Go** (intentional; no Go to delete)
- [x] Known-variable set / collect (decls, bindings, ImageSearch + ForEachRow builtins) — ✅ cutover pending — `domain/variables.rs`
- [x] Builtin/runtime variable resolve parity vs `internal/macro` (monitor builtins) — ✅ cutover pending — `monitor_builtin_var_names` + executor `apply_monitor_sizes` (Xinerama sizes; fallback virtual bounds)

### Serialize

- [x] Action + macro YAML codecs — ✅ cutover pending — `sqyre-serialize` ↔ `internal/models/serialize`
  - Notes: loads same `~/.sqyre/db.yaml`; includes `while` / `navigateselect` / `navigatekey`. Legacy `calculate` YAML decodes as `setvariable`. ImageSearch dropped unused `rowsplit`/`colsplit` (legacy keys ignored).

### Validate

- [x] Entity / variable names, search-area bounds, item grid — ✅ cutover pending — `sqyre-validate` ↔ `internal/validation`
- [x] `ValidateAction` parity (bindings + Key / Set / Pause + expression structure) — ✅ cutover pending — `validate_action` + entry validators ↔ `ValidateAction` / `macro.Validate*`
  - Notes: `EntryValidation` / unknown-var warnings in crate; UI VarEntry live warnings still partial.

### Persist / config

- [x] `db.yaml` Database load/save — ✅ cutover pending — `sqyre-persist` ↔ `internal/models/repositories` + serialize
- [x] Program catalog CRUD (+ item rename moves icon/variant files) — ✅ cutover pending — `persist/programs.rs` ↔ program repos
- [x] Paths / `sqyre_dir` / `AutoPic` — ✅ cutover pending — `sqyre-persist` ↔ `internal/config`
- [x] User settings file load/save + color prefs — ✅ cutover pending — `persist/settings.rs` ↔ settings prefs
  - Notes: library ✅; settings **UI** polish still 🟡 (see App table). Glance disposition matches library, not panel polish.

### Match / vision

- [x] Template match (CCOEFF_NORMED, mask, peaks/dedup) — ✅ cutover pending — `sqyre-match` ↔ gocv/`services` image search path
- [x] Find-pixel, match façade, OCR preprocess + Tesseract — ✅ cutover pending — `sqyre-vision` ↔ `internal/vision` + OCR
- [x] ImageSearch blurred-template + mask cache — ✅ cutover pending — `vision/search_cache.rs` ↔ Go `search_cache.go` (mtime + blur/size keys; invalidate on icon/mask path changes)
  - Notes: PureCV vs GoCV — behavioral re-prove on real macros; tessdata via `SQYRE_TESSDATA`.

### Executor / automation

- [x] Injected backends (`AutomationBackend`, capturer, matcher, focuser, OCR) — ✅ cutover pending — `sqyre-executor` ↔ `internal/services`
- [x] Flow: loop / while / break / continue / conditional / runmacro / foreach — ✅ cutover pending
- [x] Mouse/keyboard/type/wait/pause/set/save var / focus / image / pixel / OCR — ✅ cutover pending (Linux); Set evaluates `${refs}` and arithmetic expressions
- [x] `NavigateSelect` execute (+ `NavigateKey` chord branches) — ✅ `executor/navigate.rs` (grid via `CoordinateResolver::collection_grid`)
- [x] Interruptible stop/delay (+ gated post-action delay) — ✅ `interruptible_sleep`; Wait/Type/retry/RWF; skip delay after Stopped/errors
- [x] ImageSearch `repeatwhilefound` honors `max_iterations` — ✅ (optional timeout still caps)
- [x] Live runtime-var publish sink — ✅ cutover pending — `executor/runtime_vars.rs` (`SharedRuntimeVars` / `RuntimeVarSink`) ↔ Go `runtime_vars`
- [x] OCR empty-target wait — ✅ cutover pending — `ocr_target_matched` matches Go `strings.Contains` (blank target always matches)
- [ ] Cross-check remaining delay/retry/highlight/log edge cases vs Go — 🟡 mostly aligned

### Input

- [x] Linux automation (rustautogui + clipboard) — ✅ cutover pending — `sqyre-input` ↔ `services/automation.go` / robotgo
- [ ] Windows / mac automation — ❌ needs work

### Capture / focus

- [x] Linux X11 capture + focus — ✅ cutover pending — `sqyre-capture` ↔ `internal/capture` + `screen` + window_*
- [x] X11 selection outline (recording HUD rects) — ✅ cutover pending — `SelectionOutline` + app `recording_overlay` ↔ `ui/recording`
- [x] Recording coords HUD when app is hidden — ✅ cutover pending — always-on-top deferred viewport + poller `request_repaint` while armed
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

Rust boots via `make` / `make run` → `./bin/sqyre` (or `cargo run -p sqyre-app` → `./rust/target/debug/sqyre`). Legacy Go: `make go` → `./bin/sqyre-go` (`cmd/sqyre` → `internal/app.Run()`). **Do not delete Go entrypoint** until cutover gate + packaging switch.

| Feature                                                          | Go                              | Rust                                                                     | Status | Disposition     |
| ---------------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------------ | ------ | --------------- |
| Shell / single-instance / tray                                   | `internal/app`, `ui` tray       | `main`, `single_instance`, `tray`                                        | 🟡     | needs work      |
| Macro list + tree + run/stop                                     | `ui/macro`                      | `main`, tree_*                                                           | 🟡     | needs work      |
| Macro name / delay / tags                                        | `ui/macro` meta                 | `macro_meta.rs`                                                          | 🟡     | needs work      |
| Tree DnD                                                         | `ui/macro` tree_dnd*            | `tree_dnd.rs`                                                            | 🟡     | needs work      |
| Tree undo/history                                                | macro undo                      | `tree_history.rs`                                                        | 🟡     | needs work      |
| Tree clipboard                                                   | `tree_clipboard.go`             | `tree_clipboard.rs` cut/copy/paste                                       | 🟡     | needs work      |
| Action tooltips view/edit                                        | `ui/macro/action_tooltip_*`     | `action_tooltip/` (+ `sections`, NavigateSelect/Key editors)             | 🟡     | needs work      |
| Theme (dark + Sqyre gold)                                        | `ui/theme.go`                   | `theme.rs`                                                               | 🟡     | needs work      |
| Native file/folder dialogs                                       | Fyne / OS pickers               | `file_dialogs.rs` (rfd + Tokio enter)                                    | 🟡     | needs work      |
| Var pills / VarEntry                                             | `ui/custom_widgets`             | `var_pills.rs` (+ validate helpers)                                      | 🟡     | needs work      |
| Entity pickers / recording overlays                              | `ui` pickers, `ui/recording`    | `pickers`, X11 `SelectionOutline` + `recording_overlay` (coords HUD when hidden), `hotkey_record`, `key_record` (Pause continue chord), `pixel_color` (FindPixel screen sample) | 🟡     | needs work      |
| Preview tooltips                                                 | custom_widgets / action_preview | `preview_tooltip.rs`                                                     | 🟡     | needs work      |
| Data editor (programs/items/masks/collections/coords + variants) | `ui/editor`                     | `data_editor.rs` + `icon_variants.rs` (New point/SA auto-arms record + save) | 🟡     | needs work      |
| Settings panel (prefs, paths, fonts, colors)                     | `ui/settings.go`                | `settings.rs` + `persist/settings.rs` + theme                            | 🟡     | needs work      |
| Action logs UI                                                   | macro log popup                 | `action_logs_ui.rs` (incl. clear)                                        | 🟡     | needs work      |
| Variables panel + runtime vars                                   | `macro_variables`, runtime_vars | `variables_panel.rs` + `SharedRuntimeVars` (decls CRUD + live/last snapshot) | 🟡     | needs work      |
| Add-action picker / colors / icons                               | `ui/mainmenu.go`                | `add_action.rs` + `domain/blank.rs` (21 kinds, Ctrl+A; hover edits persisted defaults) | 🟡     | needs work      |
|                                                                  |                                 |                                                                          |        |                 |
| Default binary = Rust                                            | `make go` → `sqyre-go`          | `make` → `bin/sqyre` (`sqyre-app`)                                       | ✅     | done (local)    |

---

## Platform matrix

- [x] Linux X11 — 🟡 primary Rust target (automation + capture)
- [ ] Wayland — ❌ / 🟡 needs work
- [ ] Windows — ❌ needs work (Go has window/overlay paths; release still ships Go)
- [ ] macOS — ❌ needs work
- [ ] Headless test story (Null* backends / no hooks) — 🟡 present; keep in sync with Go `nohook`

---

## Cutover gate (all must be true before deleting Go runtime)

- [x] Rust is default `make` / local `./bin/sqyre`
- [ ] Shipped binary (CI AppImage / Windows release) is Rust
- [x] Macro hotkey launch works
- [ ] Data editor + settings + recording pickers usable for daily macros
- [x] Variables panel: decls CRUD + live runtime snapshot
- [x] Recording coords HUD when app is hidden during point/search-area pick
- [x] NavigateSelect implemented (with NavigateKey subaction branches)
- [x] ValidateAction parity (or Go checks explicitly dropped)
- [ ] Real-user smoke: load existing `db.yaml`, run macros, Esc/failsafe
- [ ] Then delete Go packages/files that Rust owns; mark 🔪 here

---

## Top remaining priorities

1. **UI daily-driver polish** — VarEntry live validation in data editor; collection-cell zoom/pan; remaining picker polish
2. **Real-user smoke** — load `~/.sqyre/db.yaml`, run macros, Esc/failsafe
3. **Release cutover** — CI / AppImage / Windows → Rust (still Go today)
4. **Then 🔪** — delete `internal/`, `ui/`, `cmd/sqyre` once gate clears

---

## Map (quick)


| Go                                           | Rust              |
| -------------------------------------------- | ----------------- |
| `internal/varref`                            | `sqyre-varref`    |
| `internal/models` (+ actions)                | `sqyre-domain`    |
| `internal/models/serialize`                  | `sqyre-serialize` |
| `internal/validation`                        | `sqyre-validate`  |
| `internal/config` + repos                    | `sqyre-persist`   |
| `internal/services` executor                 | `sqyre-executor`  |
| image match                                  | `sqyre-match`     |
| `internal/vision`                            | `sqyre-vision`    |
| automation / robotgo                         | `sqyre-input`     |
| `internal/capture` + `screen`                | `sqyre-capture`   |
| `macrohotkey` / `hookkeys` / `hotkeytrigger` | `sqyre-hotkeys`   |
| `ui/` + `cmd/sqyre`                          | `sqyre-app`       |


See also: [README.md](./README.md), [docs/DEVELOPING.md](../docs/DEVELOPING.md).
