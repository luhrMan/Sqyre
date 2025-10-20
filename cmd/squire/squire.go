package main

import (
	"Squire/binders"
	"Squire/internal/assets"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"

	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
)

func main() {
	go services.StartHook()
	services.FailsafeHotkey()
	repositories.ViperSerializer.Decode() // read config.yaml data and save into GO structs
	binders.InitPrograms()

	a := app.NewWithID("Squire")
	a.Settings().SetTheme(theme.DarkTheme())
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Squire")
	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()

	systemTraySetup(w)
	//Initialize ui 		(provide an object for each property of ui)
	// construct the initialized 	(add widgets to ui)
	// set bindings			(set bindings for ui widgets)
	ui.InitializeUi(w)
	ui.GetUi().ConstructUi()
	BindUi()
	w.SetContent(ui.GetUi().MainUi.CanvasObject)
	w.RequestFocus()
	w.ShowAndRun()

	services.CloseTessClient()

	err := repositories.EncodePrograms(repositories.GetPrograms())
	if err != nil {
		log.Println(err)
	}
	err = repositories.EncodeMacros(repositories.GetMacros())
	if err != nil {
		log.Println(err)
	}
}

func BindUi() {
	binders.SetActionTabBindings()
	binders.SetMacroUi()
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
