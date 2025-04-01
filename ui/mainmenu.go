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
)

func (u *Ui) createMainMenu() *fyne.MainMenu {
	// ocrActionMenuItem.Icon, _ = fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")
	macroMenu := fyne.NewMenu("Macro")
	programSelectSubMenu := fyne.NewMenuItem("Select Program", nil)
	actionSubMenu := fyne.NewMenuItem("Add Action", nil)
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
			t, err := u.GetMacroTabMacroTree()
			if err != nil {
				log.Println(err)
				return
			}
			t.Macro.Root.AddSubAction(a)
			t.Tree.Refresh()
		}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Wait", func() { addActionAndRefresh(actions.NewWait(time)) }),
		fyne.NewMenuItem("Mouse Move", func() { addActionAndRefresh(actions.NewMove(moveX, moveY)) }),
		fyne.NewMenuItem("Click", func() { addActionAndRefresh(actions.NewClick(actions.LeftOrRight(button))) }),
		fyne.NewMenuItem("Key", func() { addActionAndRefresh(actions.NewKey(key, actions.UpOrDown(state))) }),
	)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Loop", func() { addActionAndRefresh(actions.NewLoop(count, loopName, []actions.ActionInterface{})) }),
		fyne.NewMenuItem("Image Search", func() {
			addActionAndRefresh(
				actions.NewImageSearch(
					imageSearchName,
					[]actions.ActionInterface{},
					[]string{},
					programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)))
		}),
		fyne.NewMenuItem("OCR", func() {
			addActionAndRefresh(actions.NewOcr(ocrName, []actions.ActionInterface{}, ocrTarget, programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(ocrSearchBox)))
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
			robotgo.MouseSleep = globalDelay
			robotgo.KeySleep = globalDelay
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
	)

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu, testMenu)
}
