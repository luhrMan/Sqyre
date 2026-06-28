package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/uiutil"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func actionColorPrefKey(categoryKey string) string {
	switch categoryKey {
	case actions.ActionColorKeyMouseKeyboard:
		return config.PrefActionColorMouseKeyboard
	case actions.ActionColorKeyDetection:
		return config.PrefActionColorDetection
	case actions.ActionColorKeyVariables:
		return config.PrefActionColorVariables
	case actions.ActionColorKeyMiscellaneous:
		return config.PrefActionColorMiscellaneous
	case actions.ActionColorKeyWait:
		return config.PrefActionColorWait
	case actions.ActionColorKeyDefault:
		return config.PrefActionColorDefault
	default:
		return ""
	}
}

func sampleActionTypeForColorKey(categoryKey string) string {
	switch categoryKey {
	case actions.ActionColorKeyMouseKeyboard:
		return "click"
	case actions.ActionColorKeyDetection:
		return "imagesearch"
	case actions.ActionColorKeyVariables:
		return "setvariable"
	case actions.ActionColorKeyMiscellaneous:
		return "loop"
	case actions.ActionColorKeyWait:
		return "wait"
	default:
		return ""
	}
}

func loadActionColorsFromPrefs(prefs fyne.Preferences) {
	actions.ClearAllCustomActionColors()
	for _, cat := range actions.ActionColorCategories {
		prefKey := actionColorPrefKey(cat.Key)
		if prefKey == "" {
			continue
		}
		hex := prefs.String(prefKey)
		if hex == "" {
			continue
		}
		c, ok := uiutil.HexToColor(hex)
		if !ok {
			continue
		}
		r, g, b, a := c.RGBA()
		actions.SetCustomActionColor(cat.Key, color.NRGBA{
			R: byte(r >> 8),
			G: byte(g >> 8),
			B: byte(b >> 8),
			A: byte(a >> 8),
		})
	}
}

func saveActionColorPref(prefs fyne.Preferences, categoryKey string, c color.Color) {
	prefKey := actionColorPrefKey(categoryKey)
	if prefKey == "" {
		return
	}
	prefs.SetString(prefKey, uiutil.ColorToHex(c))
	actions.SetCustomActionColor(categoryKey, colorToNRGBA(c))
}

func clearActionColorPref(prefs fyne.Preferences, categoryKey string) {
	prefKey := actionColorPrefKey(categoryKey)
	if prefKey == "" {
		return
	}
	prefs.SetString(prefKey, "")
	actions.ClearCustomActionColor(categoryKey)
}

func colorToNRGBA(c color.Color) color.NRGBA {
	r, g, b, a := c.RGBA()
	return color.NRGBA{R: byte(r >> 8), G: byte(g >> 8), B: byte(b >> 8), A: byte(a >> 8)}
}

func currentActionColor(categoryKey string) color.Color {
	sample := sampleActionTypeForColorKey(categoryKey)
	return actions.ActionPastelColor(sample)
}

func refreshOpenMacroActionColors() {
	u := GetUi()
	if u == nil || u.MainUi == nil || u.MainUi.Mui == nil {
		return
	}
	u.MainUi.Mui.MTabs.RefreshActionDisplayColors()
}

func buildActionColorSettings(parent fyne.Window, prefs fyne.Preferences) fyne.CanvasObject {
	loadActionColorsFromPrefs(prefs)

	rows := make([]fyne.CanvasObject, 0, len(actions.ActionColorCategories)+1)
	swatchByKey := map[string]*canvas.Rectangle{}

	for _, cat := range actions.ActionColorCategories {
		swatch := canvas.NewRectangle(currentActionColor(cat.Key))
		swatch.SetMinSize(fyne.NewSize(28, 28))
		swatch.CornerRadius = 4
		swatch.StrokeColor = theme.Color(theme.ColorNameSeparator)
		swatch.StrokeWidth = 1
		swatchByKey[cat.Key] = swatch

		label := widget.NewLabel(cat.Label)
		label.Alignment = fyne.TextAlignLeading

		categoryKey := cat.Key
		chooseBtn := widget.NewButton("Choose…", func() {
			picker := dialog.NewColorPicker("Action color", cat.Label, func(c color.Color) {
				saveActionColorPref(prefs, categoryKey, c)
				swatch.FillColor = c
				swatch.Refresh()
				refreshOpenMacroActionColors()
			}, parent)
			picker.Advanced = true
			picker.SetColor(currentActionColor(categoryKey))
			picker.Show()
		})
		resetBtn := widget.NewButton("Reset", func() {
			clearActionColorPref(prefs, categoryKey)
			swatch.FillColor = currentActionColor(categoryKey)
			swatch.Refresh()
			refreshOpenMacroActionColors()
		})

		row := container.NewBorder(nil, nil, label, container.NewHBox(swatch, chooseBtn, resetBtn))
		rows = append(rows, row)
	}

	resetAllBtn := widget.NewButton("Reset all action colors", func() {
		for _, cat := range actions.ActionColorCategories {
			clearActionColorPref(prefs, cat.Key)
			if swatch, ok := swatchByKey[cat.Key]; ok {
				swatch.FillColor = currentActionColor(cat.Key)
				swatch.Refresh()
			}
		}
		refreshOpenMacroActionColors()
	})

	rows = append(rows, resetAllBtn)
	return container.NewVBox(rows...)
}
