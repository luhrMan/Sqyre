package ui

import (
	"image/color"

	"Sqyre/internal/config"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	kxtheme "github.com/ErikKalkoken/fyne-kx/theme"
)

var sqyrePrimary = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0xff}

// sqyreSelection is a dimmed (semi-transparent) version of sqyrePrimary for list/tree selection.
var sqyreSelection = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0x40}

// sqyreHover is a dimmed version of sqyrePrimary for menu/item hover.
var sqyreHover = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0x40}

var currentSqyreTheme *sqyreTheme

// sqyreTheme wraps the default theme and overrides the primary, selection, and hover colors.
type sqyreTheme struct {
	fyne.Theme
	fontSize float32
	uiScale  float32
}

func NewSqyreTheme() fyne.Theme {
	if currentSqyreTheme == nil {
		currentSqyreTheme = &sqyreTheme{
			Theme:    kxtheme.DefaultWithFixedVariant(theme.VariantDark),
			fontSize: config.DefaultUIFontSize,
			uiScale:  config.DefaultUIScale,
		}
	}
	return currentSqyreTheme
}

// Color returns the sqyre primary for ColorNamePrimary and ColorNameSeparator, dimmed primary for ColorNameSelection and ColorNameHover, otherwise the default theme color.
func (t *sqyreTheme) Color(name fyne.ThemeColorName, variant fyne.ThemeVariant) color.Color {
	switch name {
	case theme.ColorNamePrimary:
		return sqyrePrimary
	case theme.ColorNameSeparator:
		return sqyreSelection
	case theme.ColorNameSelection:
		return sqyreSelection
	case theme.ColorNameHover:
		return sqyreHover
	default:
		return t.Theme.Color(name, variant)
	}
}

func (t *sqyreTheme) Size(name fyne.ThemeSizeName) float32 {
	base := t.Theme.Size(name)
	scale := t.uiScale
	if scale <= 0 {
		scale = config.DefaultUIScale
	}
	fontSize := t.fontSize
	if fontSize <= 0 {
		fontSize = config.DefaultUIFontSize
	}

	switch name {
	case theme.SizeNameText:
		return fontSize
	case theme.SizeNameCaptionText:
		return fontSize * 11 / config.DefaultUIFontSize
	case theme.SizeNameHeadingText:
		return fontSize * 24 / config.DefaultUIFontSize
	case theme.SizeNameSubHeadingText:
		return fontSize * 18 / config.DefaultUIFontSize
	default:
		return base * scale
	}
}

// WrapTagChip wraps each Items-tab tag row with a border and a ~95% transparent Sqyre primary fill (5% opacity).
func WrapTagChip(inner fyne.CanvasObject) fyne.CanvasObject {
	fill := color.NRGBA{R: sqyrePrimary.R, G: sqyrePrimary.G, B: sqyrePrimary.B, A: 28}
	border := canvas.NewRectangle(fill)
	border.StrokeColor = theme.ButtonColor()
	border.StrokeWidth = 1
	border.CornerRadius = 6
	return container.NewStack(border, inner)
}

// WrapSqyreFrame wraps inner with rounded corners, a subtle Sqyre primary fill, and a stroke in the theme primary (Sqyre gold).
func WrapSqyreFrame(inner fyne.CanvasObject) fyne.CanvasObject {
	fill := color.NRGBA{R: sqyrePrimary.R, G: sqyrePrimary.G, B: sqyrePrimary.B, A: 13}
	border := canvas.NewRectangle(fill)
	border.StrokeColor = theme.Color(theme.ColorNamePrimary)
	border.StrokeWidth = 1
	border.CornerRadius = 4
	return container.NewStack(border, inner)
}
