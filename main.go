package main

import (
	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	hook "github.com/robotn/gohook"
)

func main() {
	a := app.New()
	w := a.NewWindow("Squire")
	os.Setenv("FYNE_SCALE", "1.25")
	u := &ui{win: w, mm: map[string]*macro{"test": &macro{}}, st: &settingsTabs{tabs: &container.AppTabs{}}}
	icon, _ := fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")
	//failsafe hotkey
	go func() {
		ok := hook.AddEvents("f1", "shift", "ctrl")
		if ok {
			log.Println("Exiting...")
			os.Exit(0)
		}
	}()
	w.SetContent(u.LoadMainContent())
	a.Settings().SetTheme(theme.DarkTheme())
	w.SetIcon(icon)
	w.ShowAndRun()
}
