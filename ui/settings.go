package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// SettingsUi holds the user settings screen and its widgets.
// Add fields here as you add more settings (e.g. checkboxes, entries).
type SettingsUi struct {
	CanvasObject      fyne.CanvasObject
	GeneralSection    *widget.Card
	AppearanceSection *widget.Card
	Content           *container.Scroll
}

// constructSettings builds the user settings screen layout.
func (u *Ui) constructSettings() fyne.CanvasObject {
	u.SettingsUi.Content = container.NewScroll(nil)
	u.SettingsUi.Content.SetMinSize(fyne.NewSize(400, 300))

	// General section (scaffold)
	u.SettingsUi.GeneralSection = widget.NewCard("General", "Application and behavior options.", container.NewVBox(
		widget.NewLabel("General settings will appear here."),
	))

	// Appearance section (scaffold)
	u.SettingsUi.AppearanceSection = widget.NewCard("Appearance", "Theme and display options.", container.NewVBox(
		widget.NewLabel("Appearance settings will appear here."),
	))

	u.SettingsUi.Content.Content = container.NewVBox(
		u.SettingsUi.GeneralSection,
		u.SettingsUi.AppearanceSection,
	)

	root := container.NewBorder(
		nil, nil, nil, nil,
		u.SettingsUi.Content,
	)
	u.SettingsUi.CanvasObject = root
	return root
}
