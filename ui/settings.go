package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// SettingsUi holds the user settings screen and its widgets.
type SettingsUi struct {
	CanvasObject      fyne.CanvasObject
	GeneralSection    fyne.CanvasObject
	DataSection       fyne.CanvasObject
	AppearanceSection fyne.CanvasObject
	Content           *container.Scroll
}

// settingsInfoIcon returns a small info icon that shows the given help text on hover.
func settingsInfoIcon(tip string) fyne.CanvasObject {
	icon := ttwidget.NewIcon(theme.InfoIcon())
	icon.SetToolTip(tip)
	return icon
}

// settingsRow lays out a labeled control followed by a trailing info icon whose
// tooltip carries the explanatory text (keeps the section free of hint clutter).
func settingsRow(label string, control fyne.CanvasObject, tip string) fyne.CanvasObject {
	return container.NewHBox(widget.NewLabel(label), control, settingsInfoIcon(tip))
}

// settingsSection wraps a titled group of settings in the Sqyre gold frame with a
// bold header and a muted subtitle so each area of the screen is clearly named.
func settingsSection(title, subtitle string, rows ...fyne.CanvasObject) fyne.CanvasObject {
	header := widget.NewLabelWithStyle(title, fyne.TextAlignLeading, fyne.TextStyle{Bold: true})

	body := []fyne.CanvasObject{header}
	if subtitle != "" {
		sub := widget.NewLabel(subtitle)
		sub.Wrapping = fyne.TextWrapWord
		sub.Importance = widget.LowImportance
		body = append(body, sub)
	}
	body = append(body, widget.NewSeparator())
	body = append(body, rows...)

	return WrapSqyreFrame(container.NewPadded(container.NewVBox(body...)))
}

// constructSettings builds the user settings screen layout.
func (u *Ui) constructSettings() fyne.CanvasObject {
	u.SettingsUi.Content = container.NewScroll(nil)
	u.SettingsUi.Content.SetMinSize(fyne.NewSize(400, 300))

	prefs := fyne.CurrentApp().Preferences()

	u.SettingsUi.GeneralSection = u.buildGeneralSection(prefs)
	u.SettingsUi.DataSection = u.buildDataSection()
	u.SettingsUi.AppearanceSection = u.buildAppearanceSection(prefs)

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

// buildGeneralSection builds the General application and behavior options.
func (u *Ui) buildGeneralSection(prefs fyne.Preferences) fyne.CanvasObject {
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

	hideDuringRecording := prefs.BoolWithFallback(config.PrefHideAppDuringRecording, config.DefaultHideAppDuringRecording)
	hideDuringRecordingCheck := ttwidget.NewCheck("Hide Sqyre while recording points and search areas", func(checked bool) {
		prefs.SetBool(config.PrefHideAppDuringRecording, checked)
	})
	hideDuringRecordingCheck.SetChecked(hideDuringRecording)
	hideDuringRecordingCheck.SetToolTip("When enabled, Sqyre windows are hidden before the desktop snapshot used by the recording overlay.")

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

	dragPreviewMin := config.MinDragPreviewDebounceMs
	dragPreviewMax := 1000
	dragPreviewDebounce := prefs.IntWithFallback(config.PrefDragPreviewDebounceMs, config.DefaultDragPreviewDebounceMs)
	if dragPreviewDebounce < dragPreviewMin {
		dragPreviewDebounce = config.DefaultDragPreviewDebounceMs
	}
	dragPreviewInc := custom_widgets.NewIncrementer(dragPreviewDebounce, 25, &dragPreviewMin, &dragPreviewMax)
	dragPreviewInc.SetValue(dragPreviewDebounce)
	dragPreviewInc.OnChanged = func(v int) {
		prefs.SetInt(config.PrefDragPreviewDebounceMs, v)
	}

	return settingsSection(
		"General",
		"Application and behavior options.",
		saveMetaCheck,
		highlightCheck,
		hideDuringRecordingCheck,
		widget.NewSeparator(),
		settingsRow("Image search close-match distance (px):", closeMatchesInc,
			"Image search: ignore duplicate matches within this many pixels."),
		settingsRow("Drag preview delay (ms):", dragPreviewInc,
			"Macro tree drag preview delay (ms). How long the pointer must rest before rows shift to show the drop position."),
	)
}

// buildDataSection builds the user data and configuration files options.
func (u *Ui) buildDataSection() fyne.CanvasObject {
	sqyrePathLabel := widget.NewLabel(config.GetSqyreDir())
	sqyrePathLabel.Wrapping = fyne.TextWrapWord

	openSqyreBtn := widget.NewButtonWithIcon("Open .sqyre folder", theme.FolderOpenIcon(), func() {
		if config.IsUITestMode() {
			return
		}
		if err := services.OpenSqyreDir(); err != nil {
			dialogs.ShowErrorWithEscape(err, u.Window)
		}
	})

	return settingsSection(
		"Data",
		"User data and configuration files.",
		sqyrePathLabel,
		container.NewHBox(openSqyreBtn),
	)
}

// buildAppearanceSection builds the theme and display options.
func (u *Ui) buildAppearanceSection(prefs fyne.Preferences) fyne.CanvasObject {
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

	uiScaleInc := custom_widgets.NewFloatIncrementer(currentUIScale, 0.1, &uiScaleMin, &uiScaleMax, 1)
	uiScaleInc.SetValue(currentUIScale)
	uiScaleInc.OnChanged = func(v float64) {
		currentUIScale = v
		SetAppearance(currentFontSize, v)
	}

	actionColorsHeader := widget.NewLabelWithStyle("Macro tree action colors", fyne.TextAlignLeading, fyne.TextStyle{Bold: true})

	return settingsSection(
		"Appearance",
		"Theme and display options.",
		settingsRow("Font size:", fontSizeInc,
			"Base text size for labels, buttons, and form fields."),
		settingsRow("UI scale:", uiScaleInc,
			"Scale padding, icons, and other non-text UI elements (1.0 = default)."),
		widget.NewSeparator(),
		actionColorsHeader,
		buildActionColorSettings(u, prefs),
	)
}
