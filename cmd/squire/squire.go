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

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
)

func main() {
	utils.FailsafeHotkey()
	go utils.StartHook()

	configInit()
	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()

	ui.InitializeUi(w)
	programs.GetPrograms().InitPrograms()
	ui.GetUi().SetCurrentProgram(config.DarkAndDarker)
	ui.GetUi().ConstructUi()

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
