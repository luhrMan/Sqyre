package services

import "fyne.io/fyne/v2/widget"

var (
	ispb = &widget.ProgressBar{}
	a    = &widget.Activity{}
)

func ImageSearchProgressBar() *widget.ProgressBar {
	return ispb
}

func MacroActiveIndicator() *widget.Activity {
	return a
}
