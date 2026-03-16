//go:build android

package ui

import (
	"Squire/internal/android"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// androidPermissionsSection returns the Android permissions card for macro/background execution.
func androidPermissionsSection() fyne.CanvasObject {
	accessibilityStatus := widget.NewLabel("")
	updateStatus := func() {
		if android.IsAccessibilityEnabled() {
			accessibilityStatus.SetText("Enabled")
			accessibilityStatus.Importance = widget.SuccessImportance
		} else {
			accessibilityStatus.SetText("Not enabled")
			accessibilityStatus.Importance = widget.WarningImportance
		}
	}
	updateStatus()

	openAccessibility := widget.NewButtonWithIcon("Open Accessibility settings", theme.ViewRefreshIcon(), func() {
		android.OpenAccessibilitySettings()
	})
	openAccessibility.Importance = widget.HighImportance

	requestNotification := widget.NewButtonWithIcon("Request notification permission", theme.ConfirmIcon(), func() {
		android.RequestNotificationPermission()
	})

	openBattery := widget.NewButtonWithIcon("Open battery optimization settings", theme.SettingsIcon(), func() {
		android.OpenBatteryOptimizationSettings()
	})

	refreshBtn := widget.NewButtonWithIcon("Refresh status", theme.ViewRefreshIcon(), func() {
		updateStatus()
		accessibilityStatus.Refresh()
	})

	card := widget.NewCard(
		"Android permissions",
		"Required for macros: tap, type, screen read, and background execution.",
		container.NewVBox(
			widget.NewLabel("Accessibility (tap, type, focus, screen read):"),
			container.NewHBox(accessibilityStatus, refreshBtn),
			openAccessibility,
			widget.NewSeparator(),
			widget.NewLabel("Notifications (required for background macro execution):"),
			requestNotification,
			widget.NewSeparator(),
			widget.NewLabel("Battery (optional; helps long-running macros):"),
			openBattery,
		),
	)
	return card
}
