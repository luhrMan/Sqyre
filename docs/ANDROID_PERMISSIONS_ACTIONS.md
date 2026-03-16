# Android permissions required by Sqyre actions

This document maps each action in `internal/models/actions/` to the Android permissions and capabilities needed when macro execution is implemented on Android. It also covers **background execution** so macros can run while Sqyre is not in the foreground.

---

## Action → capability mapping

| Action | Desktop behavior | Android capability / permission |
|--------|------------------|---------------------------------|
| **Wait** | `robotgo.MilliSleep` | None. Time delay only. |
| **Move** | `robotgo.Move(x, y)` | **AccessibilityService** – dispatch touch/mouse at (x,y) (e.g. `dispatchGesture`). |
| **Click** | `robotgo.Click` | **AccessibilityService** – tap / click (left/right, hold) via gestures. |
| **Key** | `robotgo.KeyDown` / `KeyUp` | **AccessibilityService** – key events or `performGlobalAction` for Back/Home/Recents. |
| **Type** | `robotgo.Type` per character | **AccessibilityService** – inject text (e.g. `setText` on focused node or inject key events). |
| **FocusWindow** | `robotgo.FindNames`, `ActiveName` | **AccessibilityService** – list windows/activities and bring target to front; or **Usage stats** + launch/bring-to-front if using `UsageStatsManager` / intents. |
| **ImageSearch** | `robotgo.CaptureImg` + gocv match | **Screen capture**: MediaProjection (runtime consent) or **AccessibilityService** `takeScreenshot()` (API 31+). No extra manifest permission for capture; user enables capture (and optionally FGS) at runtime. |
| **OCR** | `robotgo.CaptureImg` + Tesseract | Same as ImageSearch: **Screen capture** (MediaProjection or AccessibilityService screenshot). |
| **WaitForPixel** | `robotgo.GetPixelColor` in loop | **Screen capture** at a single point (MediaProjection or AccessibilityService screenshot). |
| **Loop** | Runs sub-actions repeatedly | Inherits from sub-actions; no additional permission. |
| **RunMacro** | Executes another macro by name | Inherits from executed macro; **background execution** required if macro runs when app is not visible. |
| **SetVariable** | In-memory variable store | None. |
| **Calculate** | Expression eval, set variable | None. |
| **DataList** | Read from file or manual text | **Storage**: Uses `config.GetVariablesPath()` → on Android this is app internal storage (see `constants_android.go`), so **no storage permission** needed for app-scoped paths. If you later support external files, you’d need `READ_EXTERNAL_STORAGE` / `READ_MEDIA_*` (scoped storage). |
| **SaveVariable** | Clipboard or file | **Clipboard**: No permission needed for app’s own clipboard on modern Android. **File**: Same as DataList – app-scoped path → no permission. |

---

## Background execution (macros while Sqyre is not shown)

To allow macros (including **RunMacro** and scheduled/triggered runs) to continue when the app is in the background or the screen is off:

1. **Foreground service**
   - Declare: `android.permission.FOREGROUND_SERVICE`
   - On **Android 14+** declare the specific type:
     - If the service only runs macros (no screen capture in background):  
       `android.permission.FOREGROUND_SERVICE_SPECIAL_USE` and  
       `android:foregroundServiceType="specialUse"` on the service, with a  
       `<property android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE" android:value="Macro execution: runs user-defined automation that may capture the screen and perform taps and swipes." />`
     - If the service also captures the screen in background:  
       `android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION` and  
       `android:foregroundServiceType="mediaProjection"` on the service.
   - Show a **persistent notification** while the service runs (required for foreground services).

2. **Notification permission**
   - **Android 13+**: `android.permission.POST_NOTIFICATIONS` (runtime) so the foreground notification can be shown.

3. **Optional – battery / Doze**
   - For long-running macros, users can disable battery optimization for Sqyre. You can prompt with `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`; Play policy limits when this is allowed.

---

## Summary: permissions and capabilities

| Capability | Manifest / declaration | User action |
|------------|-------------------------|-------------|
| **Input (tap, swipe, type, key)** | AccessibilityService in manifest; `BIND_ACCESSIBILITY_SERVICE`; meta-data e.g. `canPerformGestures="true"`, `canRetrieveWindowContent="true"` | Enable Sqyre in **Settings → Accessibility** |
| **Screen capture (ImageSearch, OCR, WaitForPixel)** | If using MediaProjection in a foreground service: `FOREGROUND_SERVICE_MEDIA_PROJECTION` and `foregroundServiceType="mediaProjection"`. No permission for MediaProjection itself. | Approve **screen capture** dialog; if in background, start FGS and approve capture |
| **Focus window** | Part of AccessibilityService (list/focus windows) or usage stats if you use a different approach | Same as Accessibility (or grant usage access if required) |
| **Background macro execution** | `FOREGROUND_SERVICE`; Android 14+ `FOREGROUND_SERVICE_SPECIAL_USE` (or `FOREGROUND_SERVICE_MEDIA_PROJECTION` if capturing in background); `POST_NOTIFICATIONS` (Android 13+) | Allow **notification** (13+); optionally disable battery optimization |
| **Storage (variables/data lists)** | App-scoped paths only → **no storage permission** | None |
| **Clipboard (SaveVariable)** | Not needed for app’s own clipboard | None |

---

## Minimal manifest additions (for reference)

- **AccessibilityService**: Declare `<service>` with `android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"` and an intent filter for `android.accessibilityservice.AccessibilityService`; configure in meta-data (e.g. `canPerformGestures`, `canRetrieveWindowContent`).
- **Foreground service**: `android.permission.FOREGROUND_SERVICE`; Android 14+ add the appropriate `FOREGROUND_SERVICE_*` and `foregroundServiceType` on the service.
- **Notifications**: `android.permission.POST_NOTIFICATIONS` (Android 13+).
- **MediaProjection in FGS**: `android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION` and `android:foregroundServiceType="mediaProjection"` on the service that holds MediaProjection.

No storage or clipboard permissions are required for the current design (app-scoped storage and app clipboard).

See also: `.devcontainer/builds/android/ANDROID_INIT.md` (screen capture, device control, and background execution summary).
