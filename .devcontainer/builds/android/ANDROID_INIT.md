# Android app initialization and install

This doc summarizes how to initialize and package the app correctly on Android, based on official Android docs and Fyne’s Android lifecycle.

## Initialization order

On Android, native code (Go) runs in a process started by the system; the JVM/Activity context is not available until the app is properly started.

1. **Create the Application/Activity first**  
   The Fyne app must be created with `app.New()` (or `app.NewWithID()`) **before** any code that:
   - Uses JNI (e.g. system locale, theme, storage)
   - Creates widgets or touches Fyne theme/settings

2. **Run UI and “init” on the correct thread**  
   On Android, `main()` is not the Fyne UI thread. All app/window creation and any setup that can trigger Fyne (theme, locale, widgets) must run inside `fyne.DoAndWait(...)` so it runs on the thread that has the JVM attached.

3. **Do not rely on `init()` for Fyne-related setup**  
   Package `init()` runs at process start, possibly before or on a thread without a current JVM. If Fyne (or code that uses locale/theme) runs there, you can get:
   - **“no current JVM”** when Fyne tries to load user locales
   - **SIGSEGV (e.g. in `internal/bytealg.IndexByteString`)** from invalid memory when locale/string handling runs without a valid JVM context (and on devices with MTE, this can show as `SEGV_MTESERR`).

**Correct pattern (used in `cmd/sqyre/main_android.go`):**

```go
fyne.DoAndWait(func() {
    // 1. Create app first — establishes JVM context
    a := app.NewWithID("...")
    w := a.NewWindow("...")
    // 2. Then config, repos, log, etc.
    // 3. Then UI: InitializeUi, ConstructUi, ShowAndRun()
})
```

References:

- [Fyne #5868](https://github.com/fyne-io/fyne/issues/5868) – main not on Fyne runtime on mobile
- [Fyne #5462](https://github.com/fyne-io/fyne/issues/5462) – Android crash when accessing app before it’s started
- [Fyne AndroidContext](https://docs.fyne.io/api/v2/driver/androidcontext/) – JVM/Env/Ctx for native callbacks

## APK alignment (zipalign)

If logcat shows:

```text
Can't mmap dex file .../base.apk!classes.dex directly; please zipalign to 4 bytes. Falling back to extracting file.
```

the APK’s uncompressed entries (e.g. DEX) are not 4-byte aligned. The system can still run the app by extracting the DEX, but aligning avoids that and can reduce memory use.

- **Align to 4 bytes** (for DEX and general content):  
  `zipalign -v 4 input.apk output.apk`  
  (Sign after zipalign when using `apksigner`.)
- **Verify:**  
  `zipalign -c -v 4 app.apk`

`zipalign` is in the Android SDK build-tools (e.g. `build-tools/<version>/zipalign`). If fyne-cross does not align the APK, run zipalign as a post-build step before signing, or use a build image that includes the SDK and add this step to the Android build script.

## MTE crash (SEGV_MTESERR in IndexByteString)

On devices with **Memory Tagging Extension (MTE)** enabled (e.g. Pixel 8+, `tagged_addr_ctrl` in logcat), the app may crash at startup with:

- **signal**: 11 (SIGSEGV), **code**: 9 (SEGV_MTESERR)
- **backtrace**: `libSqyre.so (internal/bytealg.IndexByteString+...)`

**When MTE is disabled** (e.g. in Developer options), the app starts successfully. So the crash is **caused by MTE**: MTE is correctly detecting a real memory-safety bug (use-after-free or access to invalid/freed memory). Without MTE, that access is not checked and the process may not crash (the underlying bug can still cause subtle issues).

**What it means:** The CPU caught an access to memory whose tag says it is invalid or already freed (use-after-free or wrong pointer). The fault is inside Go’s `IndexByteString`, so some **string** in the process is backed by invalid or freed memory when a byte-search (e.g. `strings.IndexByte`) runs.

**Typical cause:** Locale or other JNI-backed data is first used on a thread that has no JVM. The failing or invalid result is then used as a string and later passed into code that does a byte index → MTE fires in `IndexByteString`. The proper fix is in Fyne/go-locale (ensure locale is only loaded on the JVM thread and never use a bad pointer as a Go string).

**Workaround for users:** On affected devices, **disabling MTE** allows the app to run (Developer options → turn off memory tagging if available). This does not fix the bug; it only avoids the hardware check.

**Mitigations in `cmd/sqyre/main_android.go`:**

1. Create the app and window first inside `fyne.DoAndWait`, then do config/repos and UI.
2. **Force locale init on the JVM thread** as the first line in `DoAndWait`:  
   `lang.AddTranslations(fyne.NewStaticResource("en.json", []byte("{}")))`  
   so the first `updateLocalizer()` run happens on a thread that has JVM. If the crash persists, another goroutine may still be triggering locale before that runs (Fyne startup race); the robust fix would be in Fyne or go-locale.


References: [Android MTE reports](https://source.android.com/docs/security/test/memory-safety/mte-reports).
 
## Permissions for screenshots, device control, and background execution

Sqyre will need the following capabilities when macro execution and screen capture are implemented on Android. Declare them in a custom `AndroidManifest.xml` in the project root (or wherever your Fyne build reads it); Fyne uses a custom manifest if present.

### Screen capture (screenshots)

- **MediaProjection**  
  Screen capture uses the MediaProjection API. There is no manifest permission; the user must approve a **runtime consent** (Intent from `MediaProjectionManager.createScreenCaptureIntent()`). You cannot auto-grant this; the system shows a dialog.
- **If capture runs inside a foreground service** (e.g. capturing while the app is in background):
  - Declare: `android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION`
  - In the `<service>` that does the capture, set:  
    `android:foregroundServiceType="mediaProjection"`  
  - Start the service with `ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION` before using MediaProjection (required on Android 14+).

Alternative: **AccessibilityService** `takeScreenshot()` (API 31+) can capture the screen without MediaProjection, but the user must enable your app in **Settings → Accessibility** first.

### Device control (tap, swipe, input during macros)

- **Accessibility Service**   
  To inject taps, swipes, and other input you need an `AccessibilityService` with:
  - **Manifest:** Declare the service and use `android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"` (and optionally `android:canPerformGestures="true"`, `android:canRetrieveWindowContent="true"` in the service meta-data).
  - **User action:** The user must enable your app in **Settings → Accessibility**. There is no way to do this programmatically; it is an explicit user choice. 
  - Use `dispatchGesture()` for taps/swipes and `performGlobalAction()` for Back/Home/Recents.

There is no separate “input injection” permission; the capability comes from the user enabling the accessibility service.

### Running Sqyre in the background

- **Foreground service**  
  To run macro execution (or continuous capture) while the app is in background, use a **foregrou nd service** with a persistent notification:
  - `android.permission.FOREGROUND_SERVICE`
  - On **Android 14+** you must also declare the specific type:
    - If the service uses MediaProjection:  
      `android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION` and  
      `android:foregroundServiceType="mediaProjection"` on the service.
    - If the service only runs macros (no capture in background):  
      `android.permission.FOREGROUND_SERVICE_SPECIAL_USE` and  
      `android:foregroundServiceType="specialUse"` on the service, plus a  
      `<property android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE" android:value="Macro execution: runs user-defined automation that may capture the screen and perform taps and swipes." />`  
      (Use a short, accurate description; Play may review it.)
  - **Android 13+:** `android.permission.POST_NOTIFICATIONS` (runtime) so you can show the foreground notification.
- **Battery / Doze (optional):**  
  For long-running macros, users can disable battery optimization for Sqyre in **Settings → Apps → Sqyre → Battery**. You can prompt with `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`; Play policy restricts when this is allowed.

### Summary table
 A_PROJECTION` + service type if in FGS | Approve capture dialog (and enable FGS) |
| Tap / swipe / input     | AccessibilityService (declared in manifest; `canPerformGestures` etc. in meta-data) | Enable in Settings → Accessibility |
| Background execution    | `FOREGROUND_SERVICE` + type (`mediaProjection` or `specialUse`); `POST_NOTIFICATIONS` (13+) | Grant notification permission (13+) |

Implementing these will require JNI/Java code (or a Go bridge) for MediaProjection, AccessibilityService, and the foreground service; the current Android build uses stubs for execution and capture.

## Summary

- Create the Fyne app with `app.New()` **first**, inside `fyne.DoAndWait`.
- Run all config/repos and UI setup **after** app creation, still inside `fyne.DoAndWait`.
- **Pre-initialize locale on the JVM thread** with `lang.AddTranslations(...)` right after creating the app to avoid “no current JVM” and the resulting MTE/SIGSEGV in `IndexByteString`.
- Avoid doing Fyne or JNI-dependent work in package `init()`.
- Ensure the APK is zipaligned to 4 bytes (and signed after zipalign) to avoid the DEX mmap warning and optional extraction.
