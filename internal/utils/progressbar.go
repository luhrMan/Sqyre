package utils

import "fyne.io/fyne/v2/widget"

var mpb = &widget.ProgressBar{}
var ispb = &widget.ProgressBar{}

func MacroProgressBar() *widget.ProgressBar {
	return mpb
}

func ImageSearchProgressBar() *widget.ProgressBar {
	return ispb
}
