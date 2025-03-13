package main

import (
	"Squire/internal/utils"
	"Squire/ui"

	"log"
	"os"

	"github.com/go-vgo/robotgo"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
	hook "github.com/robotn/gohook"
)

var programs = make(map[string]ui.Program)

func main() {
	a := app.NewWithID("Squire")
	w := a.NewWindow("Squire")
	go toggleMousePos()
	os.Setenv("FYNE_SCALE", "1.25")

	u := &ui.Ui{}
	u.SetWindow(w)
	u.SetMacros(map[string]*ui.Macro{"test": &ui.Macro{}})
	u.CreateSettingsTabs()
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
	utils.CloseTessClient()
}

func toggleMousePos() {
	locX, locY := robotgo.Location()
	for {
		robotgo.MilliSleep(2000)
		newLocX, newLocY := robotgo.Location()
		if locX == newLocX && locY == newLocY {
			continue
		}
		locX, locY = robotgo.Location()
		log.Println(locX-utils.XOffset, locY-utils.YOffset)
		log.Println("Current title: ", robotgo.GetTitle())
	}
}
