# Go → Rust migration checklist

Shared agent tracker. **Rust is the only binary** (`make` → `./bin/sqyre`). Go/Fyne runtime deleted (🔪). Prefer clean breaking changes (no dual-path shims).

Update boxes/status when you land or delete work. Keep notes short.

## Legend

| Mark | Meaning                                         |
| ---- | ----------------------------------------------- |
| ✅    | Feature-complete in Rust (library or UI)        |
| 🟡   | Partial / WIP / Linux-only / subset             |
| ❌    | Not started or stubbed                          |
| 🔪   | Go removed after Rust owned the path end-to-end |

| Disposition         | Meaning                                                |
| ------------------- | ------------------------------------------------------ |
| `safe to delete Go` | Rust owns it; Go unused — delete now                   |
| `cutover pending`   | (legacy) Rust parity OK; Go still imported             |
| `needs work`        | Gaps before parity                                     |
| `done`              | Complete for current shipping scope                    |

**Deleted this pass:** Entire Go runtime — `cmd/sqyre`, `ui/`, `internal/` (Go packages), `go.mod` / `go.sum`, `FyneApp.toml`, `third_party/gosseract`, Go test/packaging/Windows scripts. Brand icons + tessdata live under `assets/`. Legacy YAML still decodes (`calculate` → Set; unused ImageSearch split keys ignored).

**DB note:** `~/.sqyre/db.yaml` may contain Rust-only kinds (`while` / `navigateselect` / `navigatekey`). There is no Go app left to round-trip.

---

## Status at a glance

| Area                                                    | Status          | Disposition     |
| ------------------------------------------------------- | --------------- | --------------- |
| varref                                                  | ✅               | done (🔪 Go)    |
| domain models (+ While, NavigateSelect, NavigateKey)    | ✅               | done (🔪 Go)    |
| serialize (YAML codecs)                                 | ✅               | done (🔪 Go)    |
| validate                                                | ✅               | done (🔪 Go)    |
| persist / config / settings (library)                   | ✅               | done (🔪 Go)    |
| match (PureCV)                                          | ✅               | done (🔪 Go)    |
| vision / OCR                                            | ✅               | done (🔪 Go)    |
| executor                                                | 🟡              | needs work      |
| input (automation)                                      | 🟡 Linux        | needs work      |
| capture / window focus                                  | 🟡 Linux X11    | needs work      |
| hotkeys (Esc / failsafe / pause / screen-click / macro) | ✅ Linux         | done (🔪 Go)    |
| UI / egui app                                           | 🟡              | needs work      |
| Win / mac / Wayland                                     | ❌ / 🟡          | needs work      |
| Default binary = Rust (`make` / `./bin/sqyre`)          | ✅               | done            |
| Shipped binary (CI / AppImage)                          | ✅ Linux Rust    | done            |
| Windows release                                         | ❌ dropped       | needs work      |

---

## Libraries

### Variable grammar

- [x] `sqyre-varref` — ✅ 🔪 Go `internal/varref` deleted

### Domain models

- [x] Programs, macros, variables, coords, collections — ✅ 🔪
- [x] 21 action kinds (`While` / `NavigateSelect` / `NavigateKey`; `Calculate` merged into `Set`) — ✅ 🔪
- [x] Expression eval + Set value resolve — ✅ `domain/expr.rs` + `domain/set_value.rs`
- [x] Known-variable set / collect — ✅ `domain/variables.rs`
- [x] Builtin/runtime variable resolve (monitor builtins) — ✅

### Serialize

- [x] Action + macro YAML codecs — ✅ 🔪 — loads `~/.sqyre/db.yaml`; legacy `calculate` → `setvariable`

### Validate

- [x] Entity / variable names, search-area bounds, item grid — ✅ 🔪
- [x] `ValidateAction` parity + live tooltip/data-editor validation — ✅
  - Notes: Set tooltip includes `preview_calculate` + expression builder toolbar

### Persist / config

- [x] `db.yaml` Database load/save — ✅ 🔪
- [x] Program catalog CRUD — ✅
- [x] Paths / `sqyre_dir` / `AutoPic` — ✅ — no `images/meta` dir (debug frames in-memory)
- [x] User settings + Log Meta Images — ✅

### Match / vision

- [x] Template match / find-pixel / OCR — ✅ 🔪 — PureCV + `SQYRE_TESSDATA` / `assets/tessdata`
- [x] ImageSearch blurred-template + mask cache — ✅

### Executor / automation

- [x] Injected backends — ✅ 🔪
- [x] Flow + mouse/keyboard/type/wait/pause/set/image/pixel/OCR — ✅ (Linux)
- [x] `NavigateSelect` + `NavigateKey` — ✅
- [x] Interruptible stop/delay — ✅
- [x] Live runtime-var publish — ✅
- [x] OCR empty-target wait — ✅
- [x] Action-log meta images gated — ✅
- [ ] Cross-check remaining delay/retry/highlight/log edge cases — 🟡

### Input

- [x] Linux automation — ✅ 🔪
- [ ] Windows / mac automation — ❌ needs work

### Capture / focus

- [x] Linux X11 capture + focus + selection outline + recording HUD — ✅ 🔪
- [x] Main-monitor resolution key for catalog — ✅
- [ ] Non-Linux capturer / focuser / outline — ❌ needs work
- [ ] Wayland capture/overlays — ❌ / 🟡 needs work

### Hotkeys

- [x] Esc / failsafe / continue-wait / press-latch / screen-click / macro hotkeys — ✅ 🔪
- [x] `nohook`/NullHotkeys CI story — ✅

---

## App / UI (`sqyre-app`)

Rust boots via `make` / `make run` → `./bin/sqyre`. Go entrypoint deleted.

| Feature                                                          | Rust                                                                     | Status | Disposition     |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------ | ------ | --------------- |
| Shell / single-instance / tray                                   | `main`, `single_instance` (`reacquire`), `tray`                          | 🟡     | needs work      |
| Macro list + tree + run/stop                                     | New/Duplicate/Delete + name/tag search                                   | 🟡     | needs work      |
| Macro name / delay / tags                                        | `macro_meta.rs`                                                          | 🟡     | needs work      |
| Tree DnD / undo / clipboard                                      | `tree_dnd`, `tree_history`, `tree_clipboard`                             | 🟡     | needs work      |
| Action tooltips                                                  | `action_tooltip/` + Set `f(x)`/ops + Preview                             | 🟡     | needs work      |
| Theme / file dialogs / var pills                                 | `theme`, `file_dialogs`, `var_pills`                                     | 🟡     | needs work      |
| Entity pickers / recording                                       | `pickers` + `image_view` + X11 overlays + pixel/key/hotkey record        | 🟡     | needs work      |
| Data editor                                                      | CRUD + tag autocomplete + zoom/pan + resolution key                      | 🟡     | needs work      |
| Settings / action logs / variables / add-action                  | Log Meta Images; data-dir reacquire; decls + runtime vars; Ctrl+A        | 🟡     | needs work      |
| Default + shipped Linux binary                                   | `make` / CI AppImage                                                     | ✅     | done            |

---

## Platform matrix

- [x] Linux X11 — 🟡 primary Rust target (automation + capture) — **shipped**
- [ ] Wayland — ❌ / 🟡 needs work
- [ ] Windows — ❌ needs work (releases dropped until Rust automation/capture exist)
- [ ] macOS — ❌ needs work
- [x] Headless test story (Null* backends / no hooks) — 🟡 present

---

## Cutover gate

- [x] Rust is default `make` / local `./bin/sqyre`
- [x] Shipped binary (CI AppImage) is Rust — **Linux only**; Windows release dropped
- [x] Macro hotkey launch works
- [x] Data editor + settings + recording pickers usable for daily macros (structural parity)
- [x] Variables panel: decls CRUD + live runtime snapshot
- [x] Recording coords HUD when app is hidden during point/search-area pick
- [x] NavigateSelect implemented (with NavigateKey subaction branches)
- [x] ValidateAction parity
- [ ] Real-user smoke: load existing `db.yaml`, run macros, Esc/failsafe
- [x] Delete Go packages/files — 🔪 done

---

## Top remaining priorities

1. **Real-user smoke** — load `~/.sqyre/db.yaml`, run macros, Esc/failsafe
2. **Windows / macOS** — automation + capture + packaging when ready to ship again
3. **Wayland** — capture/overlays where feasible

---

## Map (quick)

| Former Go                                | Rust              |
| ---------------------------------------- | ----------------- |
| `internal/varref`                        | `sqyre-varref`    |
| `internal/models` (+ actions)            | `sqyre-domain`    |
| `internal/models/serialize`              | `sqyre-serialize` |
| `internal/validation`                    | `sqyre-validate`  |
| `internal/config` + repos                | `sqyre-persist`   |
| `internal/services` executor             | `sqyre-executor`  |
| image match                              | `sqyre-match`     |
| `internal/vision`                        | `sqyre-vision`    |
| automation / robotgo                     | `sqyre-input`     |
| `internal/capture` + `screen`            | `sqyre-capture`   |
| `macrohotkey` / `hookkeys` / hotkeytrigger | `sqyre-hotkeys` |
| `ui/` + `cmd/sqyre`                      | `sqyre-app`       |
| `internal/assets/icons|tessdata`         | `assets/`         |

See also: [README.md](./README.md), [docs/DEVELOPING.md](../docs/DEVELOPING.md).
