package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
)

// loadVisionDetectorPrefs applies vision worker settings from Fyne preferences.
func loadVisionDetectorPrefs() {
	prefs := fyne.CurrentApp().Preferences()
	services.ApplyVisionDetectorConfig(
		prefs.StringWithFallback(config.PrefVisionWorkerPath, ""),
		prefs.StringWithFallback(config.PrefVisionModelsDir, ""),
	)
}
