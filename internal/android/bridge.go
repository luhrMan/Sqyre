//go:build android

package android

import (
	"errors"
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver"
)

// Last tap position for Move/Click sequence (Android has no cursor).
var lastTapX, lastTapY int

func SetLastTapPosition(x, y int) {
	lastTapX, lastTapY = x, y
}

func GetLastTapPosition() (x, y int) {
	return lastTapX, lastTapY
}

// ErrAccessibilityRequired is returned when the action needs the accessibility service.
var ErrAccessibilityRequired = errors.New("enable Sqyre in Settings → Accessibility to use this action")

// PerformTap injects a tap at (x, y) via the accessibility service.
func PerformTap(x, y int) error {
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		err = performTapNative(ac.Env, ac.Ctx, x, y)
		return nil
	})
	return err
}

// KeyEvent injects a key event (key name or code; down=true for key down).
func KeyEvent(key string, down bool) error {
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		err = keyEventNative(ac.Env, ac.Ctx, key, down)
		return nil
	})
	return err
}

// TypeText types the string with optional delay between keys (ms).
func TypeText(text string, delayMs int) error {
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		err = typeTextNative(ac.Env, ac.Ctx, text, delayMs)
		return nil
	})
	return err
}

// GetPixelColor returns the hex color at (x, y) on the screen.
func GetPixelColor(x, y int) (string, error) {
	var out string
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		out, err = getPixelColorNative(ac.Env, ac.Ctx, x, y)
		return nil
	})
	return out, err
}

// SetClipboard sets the clipboard text using Fyne's window clipboard (must run on UI thread).
func SetClipboard(text string) error {
	app := fyne.CurrentApp()
	if app == nil {
		return errors.New("no app")
	}
	d := app.Driver()
	if d == nil {
		return errors.New("no driver")
	}
	done := make(chan struct{})
	fyne.DoAndWait(func() {
		wins := d.AllWindows()
		if len(wins) > 0 {
			wins[0].Clipboard().SetContent(text)
		}
		close(done)
	})
	<-done
	return nil
}

// ImageSearch captures the region and runs template matching (targets are icon paths/names).
// Returns matched points in region coordinates.
func ImageSearch(leftX, topY, w, h int, targets []string, tolerance float32) ([]struct{ X, Y int }, error) {
	var points []struct{ X, Y int }
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		points, err = imageSearchNative(ac.Env, ac.Ctx, leftX, topY, w, h, targets, tolerance)
		return nil
	})
	return points, err
}

// OCR returns text from the screen region (requires accessibility or MediaProjection).
func OCR(leftX, topY, w, h int) (string, error) {
	var out string
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		out, err = ocrNative(ac.Env, ac.Ctx, leftX, topY, w, h)
		return nil
	})
	return out, err
}

// FocusWindow brings the given window/app to the front (by name or package).
func FocusWindow(windowTarget string) error {
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		err = focusWindowNative(ac.Env, ac.Ctx, windowTarget)
		return nil
	})
	return err
}

// OpenAccessibilitySettings opens the system Accessibility settings so the user can enable Sqyre.
func OpenAccessibilitySettings() {
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			return nil
		}
		openAccessibilitySettingsNative(ac.Env, ac.Ctx)
		return nil
	})
}

// RequestNotificationPermission prompts for POST_NOTIFICATIONS (Android 13+).
func RequestNotificationPermission() {
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			return nil
		}
		requestNotificationPermissionNative(ac.Env, ac.Ctx)
		return nil
	})
}

// IsAccessibilityEnabled returns whether our accessibility service is enabled.
func IsAccessibilityEnabled() bool {
	var ok bool
	driver.RunNative(func(ctx interface{}) error {
		ac, a := ctx.(*driver.AndroidContext)
		if !a {
			return nil
		}
		ok = isAccessibilityEnabledNative(ac.Env, ac.Ctx)
		return nil
	})
	return ok
}

// WindowNames returns the list of running app/window names (for Focus Window action).
func WindowNames() ([]string, error) {
	var names []string
	var err error
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			err = errors.New("missing Android context")
			return nil
		}
		names, err = windowNamesNative(ac.Env, ac.Ctx)
		return nil
	})
	return names, err
}

func windowNamesNative(env, ctx uintptr) ([]string, error) {
	return windowNamesNativeFn(env, ctx)
}

// OpenBatteryOptimizationSettings opens the "ignore battery optimization" screen for the app.
func OpenBatteryOptimizationSettings() {
	driver.RunNative(func(ctx interface{}) error {
		ac, ok := ctx.(*driver.AndroidContext)
		if !ok {
			return nil
		}
		openBatteryOptimizationSettingsNative(ac.Env, ac.Ctx)
		return nil
	})
}

// Native input/capture; overridden by bridge_cgo.go when CGo calls into Java (SqyreBridge).
var (
	performTapNativeFn     = func(env, ctx uintptr, x, y int) error { return fmt.Errorf("%w", ErrAccessibilityRequired) }
	keyEventNativeFn       = func(env, ctx uintptr, key string, down bool) error { return fmt.Errorf("%w", ErrAccessibilityRequired) }
	typeTextNativeFn       = func(env, ctx uintptr, text string, delayMs int) error { return fmt.Errorf("%w", ErrAccessibilityRequired) }
	getPixelColorNativeFn  = func(env, ctx uintptr, x, y int) (string, error) { return "", fmt.Errorf("%w", ErrAccessibilityRequired) }
	imageSearchNativeFn    = func(env, ctx uintptr, leftX, topY, w, h int, targets []string, tolerance float32) ([]struct{ X, Y int }, error) {
		return nil, fmt.Errorf("%w", ErrAccessibilityRequired)
	}
	ocrNativeFn            = func(env, ctx uintptr, leftX, topY, w, h int) (string, error) {
		return "", fmt.Errorf("%w", ErrAccessibilityRequired)
	}
	focusWindowNativeFn    = func(env, ctx uintptr, windowTarget string) error { return fmt.Errorf("%w", ErrAccessibilityRequired) }
	windowNamesNativeFn    = func(env, ctx uintptr) ([]string, error) { return nil, nil }
)

func performTapNative(env, ctx uintptr, x, y int) error     { return performTapNativeFn(env, ctx, x, y) }
func keyEventNative(env, ctx uintptr, key string, down bool) error {
	return keyEventNativeFn(env, ctx, key, down)
}
func typeTextNative(env, ctx uintptr, text string, delayMs int) error {
	return typeTextNativeFn(env, ctx, text, delayMs)
}
func getPixelColorNative(env, ctx uintptr, x, y int) (string, error) {
	return getPixelColorNativeFn(env, ctx, x, y)
}
func imageSearchNative(env, ctx uintptr, leftX, topY, w, h int, targets []string, tolerance float32) ([]struct{ X, Y int }, error) {
	return imageSearchNativeFn(env, ctx, leftX, topY, w, h, targets, tolerance)
}
func ocrNative(env, ctx uintptr, leftX, topY, w, h int) (string, error) {
	return ocrNativeFn(env, ctx, leftX, topY, w, h)
}
func focusWindowNative(env, ctx uintptr, windowTarget string) error {
	return focusWindowNativeFn(env, ctx, windowTarget)
}
// Intent/settings functions; overridden by bridge_cgo.go when CGo is used (Android build).
var (
	openAccessibilitySettingsFn     = func(env, ctx uintptr) {}
	requestNotificationPermissionFn = func(env, ctx uintptr) {}
	isAccessibilityEnabledFn        = func(env, ctx uintptr) bool { return false }
	openBatteryOptimizationSettingsFn = func(env, ctx uintptr) {}
)

func openAccessibilitySettingsNative(env, ctx uintptr)     { openAccessibilitySettingsFn(env, ctx) }
func requestNotificationPermissionNative(env, ctx uintptr) { requestNotificationPermissionFn(env, ctx) }
func isAccessibilityEnabledNative(env, ctx uintptr) bool  { return isAccessibilityEnabledFn(env, ctx) }
func openBatteryOptimizationSettingsNative(env, ctx uintptr) {
	openBatteryOptimizationSettingsFn(env, ctx)
}
