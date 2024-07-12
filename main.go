package main

import (
	"Dark-And-Darker/gui"

	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
)

func main() {
	a := app.New()
	a.Settings().SetTheme(theme.DarkTheme())
	w := a.NewWindow("Squire")
	content := gui.LoadMainContent()
	w.SetContent(content)
	w.ShowAndRun()
}
