package main

import (
	"Squire/binders"
	"Squire/internal/assets"
	"Squire/internal/models/repositories"
	"Squire/internal/models/serialize"
	"Squire/internal/services"
	"Squire/ui"

	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/driver/desktop"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

func init() {
	go services.StartHook()
	services.FailsafeHotkey()
	serialize.Decode() // read config.yaml data and save into GO structs
	repositories.MacroRepo()
	repositories.ProgramRepo()
	binders.InitPrograms()
	a := app.NewWithID("Sqyre")
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Sqyre")
	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()

	systemTraySetup(w)

	//Initialize ui 		(provide an object for each property of ui)
	ui.InitializeUi(w)
	// construct the initialized 	(add widgets to ui)
	ui.GetUi().ConstructUi()
	// set bindings			(set bindings for ui widgets)
	bindUi()

	w.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().MainUi.CanvasObject, w.Canvas()))
	w.RequestFocus()
}

func main() {
	ui.GetUi().Window.ShowAndRun()

	services.CloseTessClient()

	err := repositories.ProgramRepo().EncodeAll()
	if err != nil {
		log.Println(err)
	}
	err = repositories.MacroRepo().EncodeAll()
	if err != nil {
		log.Println(err)
	}
}

func bindUi() {
	binders.InitBinds()
	binders.SetMacroUi()
	binders.SetActionTabBindings()
	binders.SetEditorUi()
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
