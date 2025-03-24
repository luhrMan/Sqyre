package main

import (
	"Squire/encoding"
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

func main() {
	go failsafeHotkey()
	//go toggleMousePos()
	data.ViperConfig.AddConfigPath(".")
	data.ViperConfig.SetConfigName("config")
	data.ViperConfig.SetConfigType("yaml")
	err := data.ViperConfig.ReadInConfig()
	if err != nil {
		log.Println(err)
	}

	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Squire")
	ui.InitializeUi(w)
	internal.GetPrograms().InitPrograms()
	ui.GetUi().SetCurrentProgram(data.DarkAndDarker)
	ui.GetUi().ConstructUi()

	icon, _ := fyne.LoadResourceFromPath(data.ImagesPath + "Squire" + data.PNG)
	w.SetIcon(icon)
	w.ShowAndRun()

	utils.CloseTessClient()

	err = encoding.ViperSerializer.Encode(internal.GetPrograms())
	if err != nil {
		log.Println(err)
	}
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
