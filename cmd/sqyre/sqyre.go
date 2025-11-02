package main

import (
	"Squire/binders"
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/repositories"
	"Squire/internal/models/serialize"
	"Squire/internal/services"
	"Squire/ui"
	"image"

	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/driver/desktop"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"gocv.io/x/gocv"
)

func init() {
	go services.StartHook()
	services.FailsafeHotkey()
	serialize.Decode() // read config.yaml data and save into GO structs
	repositories.MacroRepo()
	repositories.ProgramRepo()

	binders.InitPrograms()

	a := app.NewWithID("Sqyre")
	a.Settings().SetTheme(&customTheme{})
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
	w.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().MainUi.CanvasObject, w.Canvas()))
	// set bindings			(set bindings for ui widgets)
	bindUi()

	// w.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().MainUi.CanvasObject, w.Canvas()))
	w.RequestFocus()
}

func main() {
	repositories.ProgramRepo().Get(config.DarkAndDarker).GetMasks()["item-corner"] = func(f ...any) *gocv.Mat {
		rows, cols, x, y :=
			f[0].(int), f[1].(int), f[2].(int), f[3].(int)
		roi :=
			image.Rect(
				(cols/x)/2,
				(rows/y)/2,
				cols,
				rows,
			)

		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
		cmask.SetTo(gocv.NewScalar(255, 255, 255, 0))

		region := cmask.Region(roi)
		defer region.Close()
		region.SetTo(gocv.NewScalar(0, 0, 0, 0))

		return &cmask
	}

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
