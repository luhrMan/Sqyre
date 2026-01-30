package main

import (
	"Squire/binders"
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/repositories"
	"Squire/internal/models/serialize"
	"Squire/internal/services"
	"Squire/ui"
	"slices"

	"github.com/go-vgo/robotgo"
	hook "github.com/luhrMan/gohook"

	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/driver/desktop"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

func init() {
	// Initialize directory structure first
	if err := config.InitializeDirectories(); err != nil {
		log.Printf("Warning: Failed to initialize directories: %v", err)
	}

	go services.StartHook()
	services.FailsafeHotkey()

	// Initialize YAML config with proper file path
	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.SetConfigFile("../../internal/config/config.yaml")
	if err := yamlConfig.ReadConfig(); err != nil {
		log.Printf("Warning: Failed to read config file: %v", err)
	}

	serialize.Decode() // read config.yaml data and save into GO structs

	// Initialize repositories - they will load data from config.yaml
	macroRepo := repositories.MacroRepo()
	log.Printf("Initialized MacroRepository with %d macros", macroRepo.Count())

	programRepo := repositories.ProgramRepo()
	log.Printf("Initialized ProgramRepository with %d programs", programRepo.Count())

	a := app.NewWithID("Sqyre")
	a.Settings().SetTheme(&assets.CustomTheme{})
	os.Setenv("FYNE_SCALE", "1.25")

	w := a.NewWindow("Sqyre")

	w.Resize(fyne.NewSize(1000, 500))
	w.SetIcon(assets.AppIcon)
	w.SetMaster()
	// w.Canvas().SetOnTypedKey(func(e *fyne.KeyEvent) {
	// 	if e.Name == fyne.KeyEscape {
	// 		w.Show()
	// 	}
	// })

	hook.Register(hook.KeyDown, []string{"esc"}, func(e hook.Event) {
		if isWindowWithTitleActive("sqyre") {
			fyne.Do(func() {
				if ui.GetUi().ActionDialog != nil {
					ui.GetUi().ActionDialog.Hide()
				}
				log.Println("checking visibility of ui")
				if !ui.GetUi().MainUi.Navigation.Root.Visible() {
					log.Println("showing main ui")
					ui.GetUi().Navigation.Back()
				}
			})
		}
	})

	systemTraySetup(w)

	//Initialize ui 		(provide an object for each property of ui)
	ui.InitializeUi(w)
	// construct the initialized 	(add widgets to ui)
	ui.GetUi().ConstructUi()
	// set bindings			(set bindings for ui widgets)
	bindUi()

	// editor := fyne.NewMenuItem("Open Data Editor", func() {
	// 	fynetooltip.DestroyWindowToolTipLayer(ui.GetUi().Window.Canvas())
	// 	ui.GetUi().Window.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().EditorUi.CanvasObject, ui.GetUi().Window.Canvas()))
	// 	ui.GetUi().Mui.MTabs.SelectedTab().UnselectAll()
	// 	ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode = ""
	// })
	// ui.GetUi().MainMenu.Items[0].Items = append(ui.GetUi().MainMenu.Items[0].Items, editor)

	w.SetContent(fynetooltip.AddWindowToolTipLayer(ui.GetUi().MainUi.Navigation, w.Canvas()))
	w.RequestFocus()
}

func main() {
	// Get the Dark and Darker program and set up the item-corner mask
	// program, err := repositories.ProgramRepo().Get("dark and darker")
	// if err != nil {
	// 	log.Printf("Warning: Could not load %s program: %v", "dark and darker", err)
	// } else {
	// 	program.GetMasks()["item-corner"] = func(f ...any) *gocv.Mat {
	// 		rows, cols, x, y :=
	// 			f[0].(int), f[1].(int), f[2].(int), f[3].(int)
	// 		roi :=
	// 			image.Rect(
	// 				// (cols/x)/2,
	// 				// (rows/y)/2,
	// 				(cols/x)-cols,
	// 				(rows/y)-rows,
	// 				cols,
	// 				rows,
	// 			)

	// 		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
	// 		cmask.SetTo(gocv.NewScalar(255, 255, 255, 0))

	// 		region := cmask.Region(roi)
	// 		defer region.Close()
	// 		region.SetTo(gocv.NewScalar(0, 0, 0, 0))

	// 		return &cmask
	// 	}
	// }
	// mask, _ := program.ItemRepo().Get("Ancient Scroll")
	// gocv.IMWrite(config.GetMetaPath()+"mask.png", *program.GetMasks()["item-corner"](162, 108, mask.GridSize[0], mask.GridSize[1]))
	// mask, _ := program.ItemRepo().Get("Bandage")
	// gocv.IMWrite(config.GetMetaPath()+"mask.png", *program.GetMasks()["item-corner"](54, 54, mask.GridSize[0], mask.GridSize[1]))
	ui.GetUi().Window.ShowAndRun()

	services.CloseTessClient()

	// Save all repositories on shutdown
	if err := repositories.ProgramRepo().Save(); err != nil {
		log.Printf("Error saving programs: %v", err)
	}
	if err := repositories.MacroRepo().Save(); err != nil {
		log.Printf("Error saving macros: %v", err)
	}
}

func bindUi() {
	// binders.InitBinds()
	binders.SetMacroUi()
	// binders.SetActionTabBindings()
	binders.SetEditorUi()
}

func systemTraySetup(w fyne.Window) {
	if desk, ok := fyne.CurrentApp().(desktop.App); ok {
		m := fyne.NewMenu("Sqyre",
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

func isWindowWithTitleActive(targetTitle string) bool {
	pids, err := robotgo.FindIds(targetTitle)
	if err != nil || len(pids) == 0 {
		return false
	}
	log.Println(pids)
	currentPid := robotgo.GetPid()
	log.Println(currentPid)
	return slices.Contains(pids, currentPid)
}
