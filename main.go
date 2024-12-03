package main

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	hook "github.com/robotn/gohook"
	"log"
	"os"
)

func main() {
	a := app.New()
	w := a.NewWindow("Squire")

	u := &ui{win: w, mt: &macroTree{}, st: &settingsTabs{tabs: &container.AppTabs{}}}
	icon, _ := fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")
	mainMenu := fyne.NewMainMenu(fyne.NewMenu("Settings"), u.createActionMenu())
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
	w.SetMainMenu(mainMenu)
	w.ShowAndRun()
}
