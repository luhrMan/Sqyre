package ui

import (
	"Squire/internal"
	"Squire/internal/actions"
	"Squire/internal/data"
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
	// newAction := func(action actions.ActionInterface) {
	// 	a := u.getCurrentTabMacro().Macro.Root.GetAction(selectedTreeItem)
	// 	if aa, ok := a.(actions.AdvancedActionInterface); ok {
	// 		aa.AddSubAction(action)
	// 		u.getCurrentTabMacro().Tree.Refresh()
	// 		return
	// 	}
	// 	a.GetParent().AddSubAction(action)
	// 	u.getCurrentTabMacro().Tree.Refresh()
	// }
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Wait", func() { u.selectedMacroTab().addActionToTree(&actions.Wait{}) }),
		fyne.NewMenuItem("Mouse Move", func() { u.selectedMacroTab().addActionToTree(&actions.Move{}) }),
		fyne.NewMenuItem("Click", func() { u.selectedMacroTab().addActionToTree(&actions.Click{}) }),
		fyne.NewMenuItem("Key", func() { u.selectedMacroTab().addActionToTree(&actions.Key{}) }),
	)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Loop", func() { u.selectedMacroTab().addActionToTree(&actions.Loop{}) }),
		fyne.NewMenuItem("Image Search", func() { u.selectedMacroTab().addActionToTree(&actions.ImageSearch{}) }),
		fyne.NewMenuItem("OCR", func() { u.selectedMacroTab().addActionToTree(&actions.Ocr{}) }),
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
		data.CalibrateInventorySearchboxes((internal.GetPrograms())[data.DarkAndDarker].Coordinates[internal.ScreenSize{data.MainMonitorSize.X, data.MainMonitorSize.Y}])
		u.st.boundImageSearchAreaSelect.SetOptions(*data.GetSearchAreaMapKeys(*data.GetSearchAreaMap()))
	}))

	testMenu := fyne.NewMenu("Test", fyne.NewMenuItem("Add Item", func() {
		addItemWindow()

	}))

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu, testMenu)
}
