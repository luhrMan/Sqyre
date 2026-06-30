package ui

import (
	"Sqyre/internal/config"
	"Sqyre/ui/custom_widgets"
	"time"

	"fyne.io/fyne/v2"
)

const (
	appearanceApplyDebounce = 80 * time.Millisecond
	appearanceSaveDebounce  = 500 * time.Millisecond
)

var (
	pendingAppearance struct {
		fontSize int
		uiScale  float64
		dirty    bool
	}
	appearanceApplyDebouncer = custom_widgets.NewDebouncer(appearanceApplyDebounce)
	appearanceSaveDebouncer  = custom_widgets.NewDebouncer(appearanceSaveDebounce)
)

func applyAppearanceTheme(fontSize int, uiScale float64) {
	t := NewSqyreTheme().(*sqyreTheme)
	t.fontSize = float32(fontSize)
	scale := float32(uiScale)
	if scale <= 0 {
		scale = config.DefaultUIScale
	}
	t.uiScale = scale
	if app := fyne.CurrentApp(); app != nil {
		app.Settings().SetTheme(t)
	}
}

func saveAppearancePrefs(fontSize int, uiScale float64) {
	app := fyne.CurrentApp()
	if app == nil {
		return
	}
	prefs := app.Preferences()
	prefs.SetInt(config.PrefUIFontSize, fontSize)
	prefs.SetFloat(config.PrefUIScale, uiScale)
}

// SetAppearance updates font size and UI scale in memory, refreshes the theme
// after a short debounce, and persists preferences after input settles.
func SetAppearance(fontSize int, uiScale float64) {
	pendingAppearance.fontSize = fontSize
	pendingAppearance.uiScale = uiScale
	pendingAppearance.dirty = true

	appearanceApplyDebouncer.Call(func() {
		applyAppearanceTheme(pendingAppearance.fontSize, pendingAppearance.uiScale)
	})
	appearanceSaveDebouncer.Call(func() {
		if !pendingAppearance.dirty {
			return
		}
		saveAppearancePrefs(pendingAppearance.fontSize, pendingAppearance.uiScale)
		pendingAppearance.dirty = false
	})
}

// ApplyAppearanceFromPrefs loads appearance settings from preferences and applies them immediately.
func ApplyAppearanceFromPrefs() {
	app := fyne.CurrentApp()
	if app == nil {
		return
	}
	prefs := app.Preferences()
	fontSize := prefs.IntWithFallback(config.PrefUIFontSize, config.DefaultUIFontSize)
	uiScale := prefs.FloatWithFallback(config.PrefUIScale, config.DefaultUIScale)
	pendingAppearance.fontSize = fontSize
	pendingAppearance.uiScale = uiScale
	pendingAppearance.dirty = false
	applyAppearanceTheme(fontSize, uiScale)
}

// FlushAppearancePrefs applies any pending theme change and writes preferences immediately.
func FlushAppearancePrefs() {
	appearanceApplyDebouncer.Stop()
	appearanceSaveDebouncer.Stop()
	if pendingAppearance.dirty {
		applyAppearanceTheme(pendingAppearance.fontSize, pendingAppearance.uiScale)
		saveAppearancePrefs(pendingAppearance.fontSize, pendingAppearance.uiScale)
		pendingAppearance.dirty = false
	}
}
