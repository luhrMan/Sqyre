# Android port

Plan for a sideloaded Android build of the Sqyre runner. Same macro YAML, same executor, same detection. Android supplies new backends for the existing `sqyre-ports` traits. Behavior that the OS cannot provide fails with `AutomationError::Unsupported` so a desktop macro does not silently do something else.

This is a plan. No Android target exists in the tree yet.

## Parity contract

| Surface | On Android |
|---------|------------|
| Macro YAML, variables, flow (loop, while, if, for-each, run macro, wait) | Same executor |
| Image search, find pixel | Same `sqyre-match` / `sqyre-vision` once a frame exists |
| OCR | Same Tesseract (`leptess`) and `eng.traineddata`, built with the NDK |
| Capture | `MediaProjection` frames in one pixel buffer. That buffer is the virtual desktop |
| Left click, scroll, clipboard | Accessibility gesture, swipe, `ClipboardManager` |
| Move | Records the point. A later click uses it. No hover |
| Right click | Long-press at the last point |
| Middle click | `AutomationError::Unsupported("middle click")` |
| Key down / key up | `AutomationError::Unsupported` for every desktop key name. Android cannot inject hardware keys into other apps |
| Type | Writes the focused editable (`ACTION_SET_TEXT`, otherwise clipboard + `ACTION_PASTE`). Custom views that only listen for key events will not see it |
| Focus window | `process_path` is the application id. Non-empty `window_title` must match the activity label. Launch is an `Intent` |
| Pause continue-key, macro hotkeys, failsafe chord | The process cannot hear global keys. Stop and Continue are notification actions plus the in-app buttons |
| Overlay buttons | `TYPE_APPLICATION_OVERLAY` windows that start a macro |
| Tray | Ongoing foreground-service notification |
| Selection grab, ScreenCap, PixelCheck | Crop or sample the projection frame; the outline is an overlay |
| Recording | Accessibility touch events become move + click steps when they carry coordinates. Key recording stays off |
| Data dir, zip backups, import / export | App files directory via `set_sqyre_dir_override` |
| Self-update | `sqyre-update` stays `Unsupported`. Install is a new APK |
| Editor | Current egui shell. A later pass only fixes layouts that are unusable with touch |

Multi-monitor virtual desktops, global hooks (`rdev`, evdev, Win32 low-level hooks), X11 override-redirect overlays, and replacing the running binary are desktop-only. `sqyre-probe` stays a desktop CLI. Permission state lives in the Android settings screen.

## Shape

```
executor (unchanged)
  AutomationBackend  → AndroidAutomation
  ScreenCapturer     → AndroidCapturer
  WindowFocuser      → AndroidFocuser
  OcrEngine          → existing LeptessOcr

Kotlin shell (required; these APIs are not available to Rust)
  activity hosting eframe
  projection foreground service + ImageReader
  accessibility service
  overlay windows
  notification actions (Stop, Continue, macro list)
```

Add `crates/sqyre-android` as the only JNI boundary. Capture, input, hotkeys, and overlay call it. They do not each link `jni` or embed Kotlin. Macro logic stays in the existing crates.

`native-runtime` stays the feature that pulls executor, vision, match, and capture. X11, Win32, ksni, tray-icon, and evdev are already cfg-gated to Linux or Windows. `sqyre-input` still depends on `rustautogui` and `arboard` for every target; those become Linux/Windows dependencies, with the Android backend beside them. `sqyre-app`’s `not(wasm32)` block must not pull `fs2` or `sqyre-update` into the Android build. Platform code uses `#[cfg(target_os = "android")]` modules, same as `linux/` and `win_*` in `sqyre-capture`.

JNI calls run on a dedicated thread that is attached to the VM. The executor worker sends commands and waits. The projection service publishes the latest frame; `capture_rect_rgb` copies out of that buffer and implements the RGB path directly (the `ScreenCapturer` default RGBA round-trip is not the production path).

## Coordinate and input mapping

Create the virtual display at the device’s real pixel size so gesture coordinates and frame coordinates match. `virtual_bounds` is that frame. `capture_monitor` is display `0` only; any other index returns `CaptureError::UnsupportedPlatform`. Rotation recreates the display and clears the cached frame. Absolute points from a previous orientation are stale, the same way a desktop resolution change is stale. The status line says when the display was recreated.

`move_to` stores `(x, y)` and does not emit a gesture. A left click that presses and releases in one step is a tap at that point. A click split into separate down and up actions returns `Unsupported`: an accessibility gesture cannot hold the pointer open across executor steps. Scroll is a short swipe. Smooth move and the following click stay two executor calls; the tap uses the stored point.

`type_char` buffers characters until a short idle or the macro leaves the type action, then commits the string once. Per-character `delay_ms` still paces the buffer. If no editable node is focused, the call returns `AutomationError::Backend`.

Focus: empty `process_path` is `InvalidArg`. No matching installed package, or a title that does not match, is `WindowNotFound`. The desktop error text already names path and title; keep it. The UI label for the field can say “package” on Android while the YAML key stays `process_path`.

Failsafe stays Ctrl+Alt+Shift+Esc in the domain so desktop files still validate. The Android listener is the notification Stop action, wired to the same stop flag the Esc hook uses on desktop.

## Phases

Each phase leaves `make test` green on the desktop host. Android modules are `cfg`-gated, so Linux and Windows builds do not link them.

### 1. Editor APK

`make android` produces a debug APK for `aarch64-linux-android` with `--no-default-features` (same editor surface as `make wasm`). Entry is eframe’s Android `android_main` in `crates/sqyre-app`. On startup, set the data dir to the app files directory before any persist call.

Exit: the APK installs on an emulator, opens the editor, and round-trips `db.yaml` through the system file picker.

Also in this phase, because it is a new release target: Android SDK, NDK, and `cargo-ndk` in `.devcontainer/Dockerfile` (or a nested image invoked by `make android`, with the Docker socket already in the devcontainer), a note in `docs/DEVELOPING.md`, and `cargo check --target aarch64-linux-android -p sqyre-app --no-default-features`.

### 2. Capture

Kotlin projection service. `AndroidCapturer` implements `shared_capturer`, `SharedRunCapturer`, and `capture_rect_rgb`. Settings explain the consent dialog and show a live permission state. Lost projection (user revoke, display change) surfaces `CaptureError` on the next search instead of reusing a stale frame.

Exit: ScreenCap saves a PNG of another app’s current screen after the user accepts the system dialog. PixelCheck reads a pixel from that frame.

### 3. Detection

Turn `native-runtime` on for the Android build. Vendor or cross-build Leptonica and Tesseract in `scripts/android/`, install headers and libs where the `leptess` build script can see them, and ship `assets/tessdata/eng.traineddata` inside the APK. OCR preprocess stays in `sqyre-vision`.

Exit: an image-search step and an OCR step run against the live projection and hit the same executor code as desktop. Host tests for `sqyre-match` stay the correctness gate; a device check confirms the frame path.

### 4. Pointer, text, clipboard

`AndroidAutomation` in `sqyre-input` behind `target_os = "android"`. Wire it from `app_backends::os_automation` and `app_run.rs` the way `OsAutomation` is wired on desktop. Accessibility must be on before Run; otherwise start fails with a status line that opens the system accessibility screen.

Exit: a macro moves, left-clicks, scrolls, and types into a focused text field of another app. Middle click and any `key` action end the step with `Unsupported`. Clipboard save works.

### 5. Launch, overlay, stop, record

`AndroidFocuser` as `OsWindowFocuser`. Overlay buttons start macros. The foreground notification is the tray: show/hide is irrelevant for a single activity; Stop, Continue, and the running macro name are the actions. Recording writes move and click steps from accessibility touch events.

Exit: Focus Window launches a package. An overlay button runs a macro. Stop on the notification halts it. Pause ends when the user hits Continue.

### 6. Touch layout

Only layouts that fail on a phone: toolbar overflow, pickers that assume a mouse drag, dialogs that ratchet off-screen. Reuse the existing egui helpers. Keep the desktop layout on a wide window.

Exit: create a macro, add an image search, run it, and stop it on a phone-sized emulator without a hardware keyboard.

## Build and packaging

| Target | Output |
|--------|--------|
| `aarch64-linux-android` | The APK that ships |
| `x86_64-linux-android` | Emulator builds |

Minimum SDK is whatever eframe 0.36’s `android-activity` and `MediaProjection` need together (API 29 or newer; pin the exact level when the shell is created). One activity, `singleTask`. Release builds are signed sideload APKs under `bin/`. No Play listing and no store metadata in this plan.

`sqyre-app`’s non-wasm dependency block currently pulls `sqyre-input`, `sqyre-update`, and `winit` for every native target. Split that so Android gets `sqyre-android` and the Android input backend, and desktop keeps `fs2` and `sqyre-update`.

## Tests

- Desktop: `make fmt`, `make check`, `make clippy`, `make test` on every phase that touches Rust.
- Android compile: `cargo ndk -t arm64-v8a check` for `sqyre-app` with `native-runtime` once phase 3 lands.
- Pure tests, no device: frame-to-`DesktopRect` mapping, package + title match, and the key / middle-click `Unsupported` policy.
- Device or emulator, by hand, at the exit criteria above. Emulator images do not join CI in the first pass.

## Out of scope

- iOS
- A Play Store build
- ML Kit or any OCR engine other than Tesseract
- Remapping desktop key names onto Back, Home, or volume keys
- Changing macro YAML to an Android-specific schema
- Shipping macOS capture as part of this work
