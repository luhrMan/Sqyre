package main

import (
	"Squire/binders"
	"Squire/encoding"
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/utils"
	"Squire/ui"

	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
)

func main() {
	go utils.StartHook()
	utils.FailsafeHotkey()

	configInit()

	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()

	systemTraySetup(w)
	ui.InitializeUi(w)
	ui.GetUi().ConstructUi()
	BindUi()

	w.RequestFocus()
	w.ShowAndRun()

	utils.CloseTessClient()

	err := encoding.ViperSerializer.Encode(binders.GetPrograms())
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
	binders.InitPrograms()
}

func BindUi() {
	binders.SetMacroUi()
	binders.SetPointsLists(ui.GetUi().ActionTabs.PointsAccordion)
	binders.SetEditorTabs()
}

func systemTraySetup(w fyne.Window) {
	if desk, ok := fyne.CurrentApp().(desktop.App); ok {
		m := fyne.NewMenu("Squire",
			fyne.NewMenuItem("Show", func() {
				w.Show()
			}))
		desk.SetSystemTrayMenu(m)
		desk.SetSystemTrayIcon(assets.AppIcon)
	}

	w.SetCloseIntercept(func() {
		w.Hide()
	})
}
