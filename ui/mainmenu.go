package ui

import (
	"Squire/internal"
	"Squire/internal/actions"
	"Squire/internal/data"
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
			u.selectedMacroTab().Macro.Root.AddSubAction(a)
			u.selectedMacroTab().Tree.Refresh()
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
			addActionAndRefresh(actions.NewImageSearch(imageSearchName, []actions.ActionInterface{}, []string{}, *data.GetSearchArea(searchArea)))
		}),
		fyne.NewMenuItem("OCR", func() {
			addActionAndRefresh(actions.NewOcr(ocrName, []actions.ActionInterface{}, ocrTarget, *data.GetSearchArea(ocrSearchBox)))
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

	calibrationMenu := fyne.NewMenu("Calibration", fyne.NewMenuItem("Calibrate Everything", func() {
		data.CalibrateInventorySearchboxes((*internal.GetPrograms())[data.DarkAndDarker].Coordinates["2560x1440"])
		u.st.boundImageSearchAreaSelect.SetOptions(*data.GetSearchAreaMapKeys(*data.GetSearchAreaMap()))
	}))

	testMenu := fyne.NewMenu("Test",
		fyne.NewMenuItem("Add Item", func() { addItemWindow() }),
		fyne.NewMenuItem("print subaction UIDS", func() {
			for _, a := range u.selectedMacroTab().Macro.Root.SubActions {
				log.Println("UID: ", a.GetUID())
			}
		}),
	)

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu, testMenu)
}
