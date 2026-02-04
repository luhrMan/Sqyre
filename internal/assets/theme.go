package assets

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type CustomTheme struct{}

var _ fyne.Theme = (*CustomTheme)(nil)

func (m CustomTheme) Color(name fyne.ThemeColorName, variant fyne.ThemeVariant) color.Color {
	switch name {
	case theme.ColorNameBackground:
		if variant == theme.VariantLight {
			// return color.White
		}
		// return color.Black
	}

	return theme.DefaultTheme().Color(name, variant)
}

func (m CustomTheme) Icon(name fyne.ThemeIconName) fyne.Resource {
	// if name == theme.IconNameHome {
	// 	return fyne.NewStaticResource("myHome", homeBytes)
	// }

	minus, _ := fyne.LoadResourceFromPath("../../internal/assets/icons/minus.svg")
	plus, _ := fyne.LoadResourceFromPath("../../internal/assets/icons/plus.svg")

	switch name {
	// case theme.IconNameMoveUp:
	// 	return moveup
	case theme.IconNameMoveDown:
		return minus
	case theme.IconNameNavigateNext:
		return plus
	}
	return theme.DefaultTheme().Icon(name)
}

func (m CustomTheme) Font(style fyne.TextStyle) fyne.Resource {
	return theme.DefaultTheme().Font(style)
}

func (m CustomTheme) Size(name fyne.ThemeSizeName) float32 {
	return theme.DefaultTheme().Size(name)
}
