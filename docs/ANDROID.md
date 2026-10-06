# Android port

A sideloaded Android build of the Sqyre runner. It uses the same macro YAML, executor and detection as desktop. Android provides new backends for the existing `sqyre-ports` traits. When the OS cannot do something, the call fails with `AutomationError::Unsupported`, so a desktop macro never silently does something else.

**Status:** `make android` builds a runtime APK (`native-runtime,overlay-buttons`, Tesseract linked statically, `eng.traineddata` bundled) and `make android-check` compiles both the editor and runtime surfaces. Capture, input, focus, OCR, the app picker, the notification Stop/Continue actions and document picks are in the tree. None of it has been exercised on a device yet; the phases below say what each one still has to prove.

## Parity contract

| Surface | On Android |
|---------|------------|
| Macro YAML, variables, flow (loop, while, if, for-each, run macro, wait) | Same executor |
| Image search, find pixel | Same `sqyre-match` / `sqyre-vision`, once a frame exists |
| OCR | Same Tesseract (`leptess`) and `eng.traineddata`. Leptonica and Tesseract are static NDK builds; the APK's `tessdata` asset is copied to app storage on launch |
| Capture | `MediaProjection` frames at the real display size. That frame is the whole virtual desktop: one monitor at (0, 0) |
| Move | Records the point. There is no hover |
| Left click | Tap at the last point. Down then up with a move in between is a swipe. A long hold keeps its duration |
| Right click | Long-press at the last point (at least 600 ms) |
| Middle click, scroll-wheel click | `Unsupported` |
| Scroll | Quarter-screen swipe at the pointer (screen center before any move) |
| Key down / key up | `Unsupported` for every key name. Android cannot inject hardware keys into other apps |
| Type | Appends to the focused editable field with `ACTION_SET_TEXT`. Custom views that only listen for key events will not see it |
| Clipboard | `ClipboardManager` |
| Focus window | `process_path` is the app package. A non-empty `window_title` must equal the app label. The app is launched with an `Intent`. The window picker lists launchable apps (label as title, package as path); the active window is the foreground package seen by the accessibility service |
| Pause continue-key, macro hotkeys, failsafe chord | The process cannot hear global keys. Stop and Continue are actions on the screen-recording notification (Continue ends a single-key Pause; multi-key Pause waits are `Unsupported`), plus the in-app Stop button |
| Overlay buttons, tray | Not yet (phase 5) |
| Selection grab, ScreenCap, PixelCheck | Crop or sample the projection frame (phase 5 for the selection UI) |
| Recording | Not yet (phase 5) |
| Data dir, zip backups, import / export | App-private storage: `internal_data_path()` is the home for `~/.sqyre` and `~/.config/sqyre`. Image and zip picks use the Storage Access Framework; the chosen document is copied into the app cache first. Folder picks are not offered |
| Self-update | Desktop-only. A new version is a new APK |
| Editor | Current egui shell. A later pass fixes layouts that are unusable with touch |

Multi-monitor desktops, global hooks (`rdev`, evdev, Win32 low-level hooks), X11 overlays and replacing the running binary are desktop-only.

## Shape

```
executor (unchanged)
  AutomationBackend  → sqyre_input::OsAutomation      (crates/sqyre-input/src/android.rs)
  ScreenCapturer     → sqyre_capture::OsCapturer      (crates/sqyre-capture/src/android_capture.rs)
  WindowFocuser      → sqyre_capture::OsWindowFocuser (same file)
  OcrEngine          → existing LeptessOcr

crates/sqyre-android (the only JNI boundary)
  FrameStore         latest projection frame + projection state (pure, host-tested)
  PointerPlanner     mouse calls → gestures (pure, host-tested)
  status             Kotlin status codes → AndroidError
  bridge             JNI calls out, natives in (android only)

android/ (Kotlin shell; these APIs are not reachable from Rust)
  MainActivity              NativeActivity hosting eframe; screen-recording consent
  ProjectionService         foreground service, MediaProjection → ImageReader → nativeOnFrame
  SqyreAccessibilityService gestures and text edits
  SqyreBridge               static methods Rust calls; status codes
```

Platform code lives in `#[cfg(target_os = "android")]` modules behind the same OS-neutral names desktop uses (`OsCapturer`, `OsWindowFocuser`, `OsAutomation`). Android is not `target_os = "linux"`, so Linux-only modules stay out without extra gating.

### Frames

The shell offers every `ImageReader` frame to Rust. Rust copies a frame only while a capture asked within the last 2 s, so an idle projection costs nothing. The first capture after an idle gap drops the stale frame and waits up to 500 ms for a new one. `capture_rect_rgb` crops straight out of the packed RGBA frame. There is no RGBA round-trip.

The shared capturer always opens. The first capture with no projection asks the shell for consent and returns `NotReady::AwaitingProjection`, so search retries until the user answers. A denial, a revoke or the shell tearing down marks the projection stopped, and captures fail with `CaptureError::Projection`. Each new macro run re-arms a stopped projection so the next capture can ask again.

### Input

An accessibility gesture is dispatched whole, so a press cannot stay open while other steps run. Down records the press. Up turns it into one gesture: a tap, a long-press as long as the hold (capped at 60 s), or a swipe if the pointer moved in between. A release with no press is a no-op.

`OsAutomation::new` fails while the accessibility service is off and opens Android's accessibility settings, so Run surfaces the error instead of dropping input.

### Stop and Continue

The screen-recording notification has Continue and Stop macro actions. Stop calls `nativeOnStopRequested`, which reaches the same `StopFlag` the desktop Esc hook uses (`sqyre_android::set_stop_handler`, installed in `SqyreApp::load`). Continue calls `nativeOnContinueRequested`, which signals `ContinueWaitBridge` (`ContinueSource::Signal`) and ends a waiting Pause step.

### File picks

Desktop, Android and WASM share one async path in `crates/sqyre-app/src/file_dialogs.rs`: a UI action calls `request(purpose, …)`, and `apply_picked_file` routes the finished pick by purpose on a later frame. Desktop answers synchronously with `rfd`. Android starts `ACTION_OPEN_DOCUMENT` and never blocks the egui thread, which would deadlock `NativeActivity` lifecycle callbacks. The shell copies the document to `cache/picked/` and calls `nativeOnDocumentPicked`; `sqyre_android::DocumentPicks` drops stale or cancelled answers.

### OCR

`scripts/android/build-ocr.sh` downloads pinned Leptonica and Tesseract sources (SHA-256 checked) and builds static libraries per ABI into `target/android/ocr/<abi>` with CMake, Ninja and the NDK toolchain file. A finished ABI is skipped on later runs. `scripts/android/ocr-env.sh` points the `leptonica-sys` / `tesseract-sys` build scripts at those `.pc` files and adds the static C++ runtime to the link. `build-apk.sh` copies `assets/tessdata` (running `make tessdata`'s download if missing) into the APK; `Tessdata.install` extracts it to `files/tessdata` and `sqyre_vision::set_tessdata_dir` makes discovery look there first.

## Phases

Each phase keeps `make fmt && make check && make test` green on the desktop host.

### 1. Editor APK

`ANDROID_FEATURES= make android` (empty) builds `sqyre-app` with `--no-default-features` (the same editor surface as `make wasm`) for `arm64-v8a`, then the Gradle shell, and copies `bin/sqyre-debug.apk`. `android_main` in `crates/sqyre-app/src/lib.rs` sets the home dir before any persist call.

Exit: the APK installs on an emulator, opens the editor, and persists `db.yaml` across restarts.

### 2. Capture

Exit: with `ANDROID_FEATURES=native-runtime,overlay-buttons`, ScreenCap saves a PNG of another app's screen after the user accepts the system dialog, and PixelCheck reads a pixel from that frame. Rotation produces frames at the new size.

### 3. Detection

Done in the build: see [OCR](#ocr). `native-runtime,overlay-buttons` is the default `ANDROID_FEATURES`.

Exit: an image-search step and an OCR step run against the live projection.

### 4. Pointer, text, clipboard

Exit: a macro moves, taps, swipes, long-presses, scrolls, and types into another app's focused text field. Middle click and any key action end the step with `Unsupported`. Clipboard writes work.

### 5. Launch, overlay, record

Done: Focus Window launches a package, the window picker lists launchable apps, Continue is on the notification, and image/zip picks use the Storage Access Framework. Still to do:
- overlay buttons as `TYPE_APPLICATION_OVERLAY` windows;
- recording from accessibility touch events.

Exit: an overlay button runs a macro, Stop on the notification halts it, and Pause ends on Continue.

### 6. Touch layout

Fix only the layouts that fail on a phone: toolbar overflow, pickers that assume a mouse drag, and dialogs that ratchet off-screen.

Exit: create a macro, add an image search, run it, and stop it on a phone-sized emulator with no hardware keyboard.

## Build and packaging

| Target | Output |
|--------|--------|
| `aarch64-linux-android` (`arm64-v8a`) | The APK that ships |
| `x86_64-linux-android` | Emulator builds (`ANDROID_ABIS="arm64-v8a x86_64"`) |

- **SDK levels:** min SDK 29, target and compile SDK 35.
- **Activity:** one `singleTask` `NativeActivity` subclass.
- **Toolchain:** the devcontainer carries JDK 17, the SDK, NDK r27, Gradle, `cargo-ndk`, CMake and Ninja.
- **Features:** `ANDROID_FEATURES` defaults to `native-runtime,overlay-buttons`; set it empty for an editor-only APK without the OCR build.
- **Stripping:** Gradle reads `ANDROID_NDK_HOME` and its `source.properties` so AGP strips `libsqyre_app.so` with the NDK `cargo-ndk` linked against.
- **Releases:** `ANDROID_PROFILE=release make android` builds an unsigned release APK. Signing and a store listing are out of scope.
- **Renderer:** eframe is pinned to Glow (GLES) on Android. wgpu would pick Vulkan, which crashes the emulator's SwiftShader host renderer.

## Emulator

Build an APK with an x86_64 library first: `ANDROID_ABIS="arm64-v8a x86_64" make android`.

| Command | Effect |
|---------|--------|
| `make android-emulator` | Boots the `sqyre` AVD (Pixel 6, API 35 `google_apis` x86_64) in a window, installs `bin/sqyre-debug.apk`, enables the accessibility service and launches the app |
| `make android-emulator-headless` | Same with no window, for agents and scripted checks |
| `make android-emulator-stop` | Stops the emulator |
| `scripts/android/emulator.sh install` | Reinstalls and relaunches after a rebuild |
| `scripts/android/emulator.sh screenshot [FILE]` | Saves the screen to `FILE` (default `bin/emulator.png`) |
| `scripts/android/emulator.sh adb ARGS...` | Runs `adb` against the emulator, e.g. `adb logcat -b crash` |

- **Where it runs:** directly when `emulator` is on `PATH` and `/dev/kvm` is usable; otherwise in a `sqyre-android-emulator` container from the devcontainer image (built on first use), with `--network host`, `/dev/kvm` and, for the window, the host X socket and `XAUTHORITY`.
- **GPU:** `SQYRE_EMULATOR_GPU` defaults to `swangle_indirect`; `swiftshader_indirect` segfaults during boot on this image.
- **Signing:** each fresh build container makes a new debug key, so `install` uninstalls first when the signature changed. App data is reset when that happens.
- **Logs:** the emulator log is `target/emulator.log` (local) or `docker logs sqyre-android-emulator`.

## Tests

- **Desktop:** `make fmt`, `make check` and `make test`. The pure `sqyre-android` modules (frame packing, frame store, gesture planning, status codes, app-list parsing, document picks, notification handlers) and the crop kernels in `sqyre-capture` run here.
- **Android compile:** `make android-check` in the devcontainer checks the editor and runtime `sqyre-app` plus `sqyre-capture`, `sqyre-input` and `sqyre-probe`. It builds the OCR libraries on first run.
- **Device or emulator:** by hand, at each phase's exit criteria, with `make android-emulator` or `make android-emulator-headless` plus `scripts/android/emulator.sh screenshot`. Emulators are not in CI yet.

## Out of scope

- iOS
- A Play Store build
- ML Kit or any OCR engine other than Tesseract
- Remapping desktop key names onto Back, Home, or volume keys
- An Android-specific macro YAML schema
