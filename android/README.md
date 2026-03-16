# Android build and permissions

This folder holds Android-specific assets for Sqyre: manifest snippet and Java sources for the Accessibility Service.

## Contents

- **AndroidManifest.xml** – Permissions and `SqyreAccessibilityService` declaration; merge with Fyne-generated manifest.
- **java/com/sqyre/app/SqyreAccessibilityService.java** – Accessibility service: tap (dispatchGesture), type (SET_TEXT on focused node), key (performGlobalAction), getPixelColor (takeScreenshot API 31+), getWindowNames, focusWindow.
- **java/com/sqyre/app/SqyreBridge.java** – Static bridge called from JNI (`internal/android/android.c`): `performTap`, `typeText`, `keyEvent`, `getPixelColor`, `getWindowNames`, `focusWindow`, `isServiceEnabled`.
- **res/xml/accessibility_service_config.xml** – Optional; if included in the build, declare `canPerformGestures` and `canRetrieveWindowContent` here. Otherwise capabilities are set in code in `onServiceConnected()`.

## Permissions and capabilities

- **Accessibility**: Required for tap, type, key, focus window, and screen read. User must enable "Sqyre" in **Settings → Accessibility**.
- **Foreground service**: For running macros in the background; declare `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_SPECIAL_USE` (Android 14+).
- **Notifications**: `POST_NOTIFICATIONS` (Android 13+) for the foreground service notification.

The in-app **Settings** screen shows an "Android permissions" card on Android with buttons to open the relevant system settings.

## Build

Ensure the app package is `com.sqyre.app` (or update the manifest and `internal/config/constants_android.go`). When building with `fyne build -os android` (or your script), include this folder’s Java and manifest so that:

1. The manifest merges with the generated one (permissions + service declaration).
2. The Java classes are compiled and included in the APK so that `internal/android/android.c` JNI calls to `com.sqyre.app.SqyreBridge` succeed.

If the Java classes are not in the build, the Go bridge falls back to returning "enable Sqyre in Settings → Accessibility" for tap/type/key/screen/focus actions.
