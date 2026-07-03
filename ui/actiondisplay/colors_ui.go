package actiondisplay

import (
	"image/color"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

func appIsDark() bool {
	if app := fyne.CurrentApp(); app != nil {
		return app.Settings().ThemeVariant() == theme.VariantDark
	}
	return false
}

func DefaultActionPastelColorForApp(actionType string) color.NRGBA {
	return actions.DefaultActionPastelColor(actionType, appIsDark())
}

func ActionPastelColorForApp(actionType string) color.NRGBA {
	return actions.ActionPastelColor(actionType, appIsDark())
}
