package main

import (
	"Dark-And-Darker/gui"
	"os"

	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
	hook "github.com/robotn/gohook"
)

func main() {
	a := app.New()
	w := a.NewWindow("Squire")
	icon, _ := fyne.LoadResourceFromPath("./images/Squire.png")

	mainMenu := fyne.NewMainMenu(fyne.NewMenu("Settings"), gui.CreateActionMenu())
	//failsafe hotkey
	go func() {
		ok := hook.AddEvents("f1", "shift", "ctrl")
		if ok {
			log.Println("Exiting...")
			os.Exit(0)
		}
	}()
	w.SetContent(gui.LoadMainContent())
	a.Settings().SetTheme(theme.DarkTheme())
	w.SetIcon(icon)
	w.SetMainMenu(mainMenu)
	w.ShowAndRun()
}
