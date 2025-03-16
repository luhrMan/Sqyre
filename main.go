package main

import (
	sen "Squire/encoding"
	"Squire/internal"
	"Squire/internal/data"
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

// var Programs = make(map[string]internal.Program)

func main() {
	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")
	go toggleMousePos()
	go failsafeHotkey()

	w := a.NewWindow("Squire")
	p := internal.GetPrograms()
	u := &ui.Ui{}
	u.SetWindow(w)
	u.SetMacroTreeMap(map[string]*ui.MacroTree{"test": &ui.MacroTree{}})
	u.CreateSettingsTabs()

	w.SetContent(u.LoadMainContent())
	icon, _ := fyne.LoadResourceFromPath("./internal/data/resources/images/Squire.png")
	w.SetIcon(icon)

	w.ShowAndRun()

	utils.CloseTessClient()
	sen.GobSerializer.Encode(p, "programData")
	log.Println(p)
}

func failsafeHotkey() {
	ok := hook.AddEvents("f1", "shift", "ctrl")
	if ok {
		log.Println("Exiting...")
		os.Exit(0)
	}
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
		log.Println(locX-data.XOffset, locY-data.YOffset)
		log.Println("Current title: ", robotgo.GetTitle())
	}
}
