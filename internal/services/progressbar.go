package services

import "fyne.io/fyne/v2/widget"

var (
	mpb  = &widget.ProgressBar{}
	ispb = &widget.ProgressBar{}
	a    = &widget.Activity{}
)

func MacroProgressBar() *widget.ProgressBar {
	return mpb
}

func ImageSearchProgressBar() *widget.ProgressBar {
	return ispb
}

func MacroActiveIndicator() *widget.Activity {
	return a
}
