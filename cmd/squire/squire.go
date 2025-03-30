package main

import (
	"Squire/encoding"
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/programs"
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

	configInit()
	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 500))

	ui.InitializeUi(w)
	programs.GetPrograms().InitPrograms()
	ui.GetUi().SetCurrentProgram(config.DarkAndDarker)
	ui.GetUi().ConstructUi()

	w.SetIcon(assets.AppIcon)
	w.SetMaster()
	w.ShowAndRun()

	utils.CloseTessClient()

	err := encoding.ViperSerializer.Encode(programs.GetPrograms())
	if err != nil {
		log.Println(err)
	}
}

func configInit() {
	config.ViperConfig.AddConfigPath("../../internal/config")
	config.ViperConfig.SetConfigName("config")
	config.ViperConfig.SetConfigType("yaml")
	err := config.ViperConfig.ReadInConfig()
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
		log.Println(locX-config.XOffset, locY-config.YOffset)
		log.Println("Current title: ", robotgo.GetTitle())
	}
}
