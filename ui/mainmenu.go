package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"github.com/go-vgo/robotgo"
	hook "github.com/robotn/gohook"
)

func (u *Ui) createMainMenu() *fyne.MainMenu {
	// ocrActionMenuItem.Icon, _ = fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")
	macroMenu := fyne.NewMenu("Macro")
	programSelectSubMenu := fyne.NewMenuItem("Select Program", nil)
	actionSubMenu := fyne.NewMenuItem("Add Blank Action", nil)
	basicActionsSubMenu := fyne.NewMenuItem("Basic Actions", nil)
	advancedActionsSubMenu := fyne.NewMenuItem("Advanced Actions", nil)

	macroMenu.Items = append(macroMenu.Items, actionSubMenu, programSelectSubMenu)
	programSelectSubMenu.ChildMenu = fyne.NewMenu("")
	actionSubMenu.ChildMenu = fyne.NewMenu("")

	actionSubMenu.ChildMenu.Items = append(actionSubMenu.ChildMenu.Items,
		basicActionsSubMenu,
		advancedActionsSubMenu,
	)
	addActionAndRefresh :=
		func(a actions.ActionInterface) {
			// t, err := u.mui.mtabs.GetTabTree()
			mt, err := ui.mui.mtabs.selectedTab()
			if err != nil {
				log.Println(err)
				return
			}

			mt.Macro.Root.AddSubAction(a)
			mt.Select(a.GetUID())
			mt.Refresh()
		}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Wait", func() { addActionAndRefresh(actions.NewWait(0)) }),
		fyne.NewMenuItem("Mouse Move", func() { addActionAndRefresh(actions.NewMove(coordinates.Point{Name: "", X: 0, Y: 0})) }),
		fyne.NewMenuItem("Click", func() { addActionAndRefresh(actions.NewClick("left")) }),
		fyne.NewMenuItem("Key", func() { addActionAndRefresh(actions.NewKey("ctrl", "down")) }),
	)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Loop", func() { addActionAndRefresh(actions.NewLoop(1, "", []actions.ActionInterface{})) }),
		fyne.NewMenuItem("Image Search", func() {
			addActionAndRefresh(actions.NewImageSearch(
				"",
				[]actions.ActionInterface{},
				[]string{},
				coordinates.SearchArea{},
			))
		}),
		fyne.NewMenuItem("OCR", func() {
			addActionAndRefresh(actions.NewOcr("", []actions.ActionInterface{}, "", coordinates.SearchArea{}))
		}),
	)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		w, h := robotgo.GetScreenSize()
		str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
		for d := range robotgo.DisplaysNum() {
			_, _, mh, mw := robotgo.GetDisplayBounds(d)
			str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
		}
		dialog.ShowInformation("Computer Information", str, u.win)
	})

	calibrationMenu := fyne.NewMenu("Coordinate Calibration",
		fyne.NewMenuItem("Everything", func() {

		}),
		fyne.NewMenuItem("Top Menu", func() {
			coordinates.TopMenuTabLocations((*programs.GetPrograms())[config.DarkAndDarker].Coordinates["2560x1440"])
		}),
		fyne.NewMenuItem("Inventories", func() {
			robotgo.MouseSleep = 0
			robotgo.KeySleep = 0
			coordinates.CalibrateInventorySearchboxes((*programs.GetPrograms())[config.DarkAndDarker].Coordinates["2560x1440"])
			u.at.boundImageSearchAreaSelect.SetOptions(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice())
			mt, err := u.mui.mtabs.selectedTab()
			if err != nil {
				log.Println(err)
				return
			}
			robotgo.MouseSleep = mt.Macro.GlobalDelay
			robotgo.KeySleep = mt.Macro.GlobalDelay
		}),
		fyne.NewMenuItem("Stash-screen", func() {

		}),
		fyne.NewMenuItem("Merchants-screen", func() {

		}),
	)

	testMenu := fyne.NewMenu("Test",
		fyne.NewMenuItem("Add Item", func() { addItemWindow() }),
		fyne.NewMenuItem("Test string slice", func() {
			log.Println("String Map:",
				config.ViperConfig.Get("programs"+"."+config.DarkAndDarker+"."+"macros"),
			)
		}),
		fyne.NewMenuItem("unregister failsafe", func() {
			fs := []string{"esc", "ctrl", "shift"}

			hook.Unregister(hook.KeyDown, fs)
		}),
	)

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu, testMenu)
}
