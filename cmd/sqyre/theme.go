package main

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type customTheme struct{}

var _ fyne.Theme = (*customTheme)(nil)

func (m customTheme) Color(name fyne.ThemeColorName, variant fyne.ThemeVariant) color.Color {
	switch name {
	case theme.ColorNameBackground:
		if variant == theme.VariantLight {
			// return color.White
		}
		// return color.Black
	}

	return theme.DefaultTheme().Color(name, variant)
}

func (m customTheme) Icon(name fyne.ThemeIconName) fyne.Resource {
	// if name == theme.IconNameHome {
	// 	return fyne.NewStaticResource("myHome", homeBytes)
	// }
	moveup, _ := fyne.LoadResourceFromPath("../../internal/assets/MoveUp.svg")
	minus, _ := fyne.LoadResourceFromPath("../../internal/assets/minus.svg")
	plus, _ := fyne.LoadResourceFromPath("../../internal/assets/plus.svg")

	switch name {
	case theme.IconNameMoveUp:
		return moveup
	case theme.IconNameMoveDown:
		return minus
	case theme.IconNameNavigateNext:
		return plus
	}
	return theme.DefaultTheme().Icon(name)
}

func (m customTheme) Font(style fyne.TextStyle) fyne.Resource {
	return theme.DefaultTheme().Font(style)
}

func (m customTheme) Size(name fyne.ThemeSizeName) float32 {
	return theme.DefaultTheme().Size(name)
}
