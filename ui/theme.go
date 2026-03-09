package ui

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

var sqyrePrimary = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0xff}

// sqyreSelection is a dimmed (semi-transparent) version of sqyrePrimary for list/tree selection.
var sqyreSelection = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0x40}

// sqyreHover is a dimmed version of sqyrePrimary for menu/item hover.
var sqyreHover = color.NRGBA{R: 0xdc, G: 0x9d, B: 0x2e, A: 0x40}

// sqyreTheme wraps the default theme and overrides the primary, selection, and hover colors.
type sqyreTheme struct {
	fyne.Theme
}

func NewSqyreTheme() fyne.Theme {
	return &sqyreTheme{Theme: theme.DefaultTheme()}
}

// Color returns the sqyre primary for ColorNamePrimary, dimmed primary for ColorNameSelection and ColorNameHover, otherwise the default theme color.
func (t *sqyreTheme) Color(name fyne.ThemeColorName, variant fyne.ThemeVariant) color.Color {
	switch name {
	case theme.ColorNamePrimary:
		return sqyrePrimary
	case theme.ColorNameSelection:
		return sqyreSelection
	case theme.ColorNameHover:
		return sqyreHover
	default:
		return t.Theme.Color(name, variant)
	}
}
