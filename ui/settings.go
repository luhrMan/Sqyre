package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/services"

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

	u.SettingsUi.GeneralSection = widget.NewCard("General", "Application and behavior options.", container.NewVBox(
		saveMetaCheck,
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

	// Appearance section (scaffold)
	u.SettingsUi.AppearanceSection = widget.NewCard("Appearance", "Theme and display options.", container.NewVBox(
		widget.NewLabel("Appearance settings will appear here."),
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
