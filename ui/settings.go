package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// SettingsUi holds the user settings screen and its widgets.
type SettingsUi struct {
	CanvasObject      fyne.CanvasObject
	GeneralSection    *widget.Card
	DataSection       *widget.Card
	AppearanceSection *widget.Card
	Content           *container.Scroll
}

// constructSettings builds the user settings screen layout.
func (u *Ui) constructSettings() fyne.CanvasObject {
	u.SettingsUi.Content = container.NewScroll(nil)
	u.SettingsUi.Content.SetMinSize(fyne.NewSize(400, 300))

	prefs := fyne.CurrentApp().Preferences()

	saveMetaCheck := widget.NewCheck("Save meta images during execution", func(checked bool) {
		prefs.SetBool(config.PrefSaveMetaImages, checked)
	})
	saveMetaCheck.SetChecked(prefs.BoolWithFallback(config.PrefSaveMetaImages, false))

	highlightEnabled := prefs.BoolWithFallback(config.PrefHighlightActiveAction, false)
	services.SetHighlightEnabled(highlightEnabled)
	highlightCheck := widget.NewCheck("Highlight the currently executing action", func(checked bool) {
		prefs.SetBool(config.PrefHighlightActiveAction, checked)
		services.SetHighlightEnabled(checked)
		if !checked {
			services.ClearHighlights()
		}
	})
	highlightCheck.SetChecked(highlightEnabled)

	closeMatchesMin := 0
	closeMatchesMax := 100
	closeMatchesDistance := prefs.IntWithFallback(config.PrefImageSearchCloseMatchesDistance, config.DefaultImageSearchCloseMatchesDistance)
	services.SetImageSearchCloseMatchesDistance(closeMatchesDistance)
	closeMatchesInc := custom_widgets.NewIncrementer(closeMatchesDistance, 1, &closeMatchesMin, &closeMatchesMax)
	closeMatchesInc.SetValue(closeMatchesDistance)
	closeMatchesInc.OnChanged = func(v int) {
		prefs.SetInt(config.PrefImageSearchCloseMatchesDistance, v)
		services.SetImageSearchCloseMatchesDistance(v)
	}
	closeMatchesHint := widget.NewLabel("Image search: ignore duplicate matches within this many pixels.")
	closeMatchesHint.Wrapping = fyne.TextWrapWord

	u.SettingsUi.GeneralSection = widget.NewCard("General", "Application and behavior options.", container.NewVBox(
		saveMetaCheck,
		highlightCheck,
		closeMatchesHint,
		closeMatchesInc,
	))

	sqyrePathLabel := widget.NewLabel(config.GetSqyreDir())
	sqyrePathLabel.Wrapping = fyne.TextWrapWord
	openSqyreBtn := widget.NewButtonWithIcon("Open .sqyre folder", theme.FolderOpenIcon(), func() {
		if config.IsUITestMode() {
			return
		}
		if err := services.OpenSqyreDir(); err != nil {
			ShowErrorWithEscape(err, u.Window)
		}
	})
	u.SettingsUi.DataSection = widget.NewCard("Data", "User data and configuration files.", container.NewVBox(
		sqyrePathLabel,
		openSqyreBtn,
	))

	fontSizeMin := 10
	fontSizeMax := 28
	uiScaleMin := 0.5
	uiScaleMax := 2.5
	currentFontSize := prefs.IntWithFallback(config.PrefUIFontSize, config.DefaultUIFontSize)
	currentUIScale := prefs.FloatWithFallback(config.PrefUIScale, config.DefaultUIScale)

	fontSizeInc := custom_widgets.NewIncrementer(currentFontSize, 1, &fontSizeMin, &fontSizeMax)
	fontSizeInc.SetValue(currentFontSize)
	fontSizeInc.OnChanged = func(v int) {
		currentFontSize = v
		SetAppearance(v, currentUIScale)
	}
	fontSizeHint := widget.NewLabel("Base text size for labels, buttons, and form fields.")
	fontSizeHint.Wrapping = fyne.TextWrapWord

	uiScaleInc := custom_widgets.NewFloatIncrementer(currentUIScale, 0.1, &uiScaleMin, &uiScaleMax, 1)
	uiScaleInc.SetValue(currentUIScale)
	uiScaleInc.OnChanged = func(v float64) {
		currentUIScale = v
		SetAppearance(currentFontSize, v)
	}
	uiScaleHint := widget.NewLabel("Scale padding, icons, and other non-text UI elements (1.0 = default).")
	uiScaleHint.Wrapping = fyne.TextWrapWord

	// Appearance section
	u.SettingsUi.AppearanceSection = widget.NewCard("Appearance", "Theme and display options.", container.NewVBox(
		fontSizeHint,
		container.NewHBox(widget.NewLabel("Font size:"), fontSizeInc),
		uiScaleHint,
		container.NewHBox(widget.NewLabel("UI scale:"), uiScaleInc),
		widget.NewLabel("Macro tree action colors"),
		buildActionColorSettings(u.Window, prefs),
	))

	u.SettingsUi.Content.Content = container.NewVBox(
		u.SettingsUi.GeneralSection,
		u.SettingsUi.DataSection,
		u.SettingsUi.AppearanceSection,
	)

	root := container.NewBorder(
		nil, nil, nil, nil,
		u.SettingsUi.Content,
	)
	u.SettingsUi.CanvasObject = root
	return root
}
