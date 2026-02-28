package ui

import (
	"Squire/internal/models/actions"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"github.com/go-vgo/robotgo"
)

func (u *Ui) constructMainMenu() *fyne.MainMenu {
	macroMenu := fyne.NewMenu("Macro")
	actionSubMenu := fyne.NewMenuItem("Add Blank Action", nil)
	basicActionsSubMenu := fyne.NewMenuItem("Basic", nil)
	advancedActionsSubMenu := fyne.NewMenuItem("Advanced", nil)
	variableActionsSubMenu := fyne.NewMenuItem("Variable", nil)

	macroMenu.Items = append(macroMenu.Items,
		actionSubMenu,
	)
	actionSubMenu.ChildMenu = fyne.NewMenu("")

	actionSubMenu.ChildMenu.Items = append(actionSubMenu.ChildMenu.Items,
		basicActionsSubMenu,
		advancedActionsSubMenu,
		variableActionsSubMenu,
	)
	addActionAndRefresh :=
		func(a actions.ActionInterface) {
			mt := u.Mui.MTabs.SelectedTab()
			if mt == nil {
				return
			}
			selectedNode := mt.Macro.Root.GetAction(mt.SelectedNode)
			if selectedNode == nil {
				selectedNode = mt.Macro.Root
			}
			if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
				s.AddSubAction(a)
			} else {
				selectedNode.GetParent().AddSubAction(a)
			}
			mt.Refresh()
			mt.Select(a.GetUID())
			mt.SelectedNode = a.GetUID()
		}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItemWithIcon("Wait", actions.NewWait(0).Icon(), func() { addActionAndRefresh(actions.NewWait(0)) }),
		fyne.NewMenuItemWithIcon("Mouse Move", actions.NewMove(actions.Point{Name: "", X: 0, Y: 0}).Icon(), func() { addActionAndRefresh(actions.NewMove(actions.Point{Name: "", X: 0, Y: 0})) }),
		fyne.NewMenuItemWithIcon("Click", actions.NewClick(false, false).Icon(), func() { addActionAndRefresh(actions.NewClick(false, false)) }),
		fyne.NewMenuItemWithIcon("Key", actions.NewKey("ctrl", true).Icon(), func() { addActionAndRefresh(actions.NewKey("ctrl", true)) }),
		fyne.NewMenuItemWithIcon("Focus window", actions.NewFocusWindow("").Icon(), func() {
			addActionAndRefresh(actions.NewFocusWindow(""))
		}),
	)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItemWithIcon("Loop", actions.NewLoop(1, "", []actions.ActionInterface{}).Icon(), func() { addActionAndRefresh(actions.NewLoop(1, "", []actions.ActionInterface{})) }),
		fyne.NewMenuItemWithIcon("Image Search", actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, actions.SearchArea{}, 1, 1, 0.95, 5).Icon(), func() {
			addActionAndRefresh(actions.NewImageSearch(
				"",
				[]actions.ActionInterface{},
				[]string{},
				actions.SearchArea{},
				1,
				1,
				0.95,
				5,
			))
		}),
		fyne.NewMenuItemWithIcon("OCR", actions.NewOcr("", []actions.ActionInterface{}, "template", actions.SearchArea{Name: "template search area"}).Icon(), func() {
			addActionAndRefresh(actions.NewOcr("", []actions.ActionInterface{}, "template", actions.SearchArea{Name: "template search area"}))
		}),
		// fyne.NewMenuItemWithIcon("Calibration", actions.NewCalibration("", "", actions.SearchArea{}, nil, 1, 1, 0.95, 5).Icon(), func() {
		// 	addActionAndRefresh(actions.NewCalibration("", "", actions.SearchArea{}, nil, 1, 1, 0.95, 5))
		// }),
		fyne.NewMenuItemWithIcon("Wait for pixel", actions.NewWaitForPixel("", actions.Point{}, "ffffff", 0, 0, nil).Icon(), func() {
			addActionAndRefresh(actions.NewWaitForPixel("", actions.Point{}, "ffffff", 0, 0, nil))
		}),
	)
	variableActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItemWithIcon("Set", actions.NewSetVariable("", "").Icon(), func() { addActionAndRefresh(actions.NewSetVariable("", "")) }),
		fyne.NewMenuItemWithIcon("Calculate", actions.NewCalculate("", "").Icon(), func() { addActionAndRefresh(actions.NewCalculate("", "")) }),
		fyne.NewMenuItemWithIcon("Read from", actions.NewDataList("", "", false).Icon(), func() { addActionAndRefresh(actions.NewDataList("", "", false)) }),
		fyne.NewMenuItemWithIcon("Save to", actions.NewSaveVariable("", "", false, false).Icon(), func() { addActionAndRefresh(actions.NewSaveVariable("", "", false, false)) }),
	)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		w, h := robotgo.GetScreenSize()
		str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
		for d := range robotgo.DisplaysNum() {
			_, _, mh, mw := robotgo.GetDisplayBounds(d)
			str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
		}
		dialog.ShowInformation("Computer Information", str, u.Window)
	})

	editor := fyne.NewMenuItem("Data Editor", func() {
		u.MainUi.Navigation.PushWithTitle(
			fynetooltip.AddWindowToolTipLayer(u.EditorUi.CanvasObject, u.Window.Canvas()),
			"Editor",
		)
		if mt := GetUi().Mui.MTabs.SelectedTab(); mt != nil {
			mt.UnselectAll()
			mt.SelectedNode = ""
		}
	})

	userSettings := fyne.NewMenuItem("User Settings", func() {
		u.MainUi.Navigation.PushWithTitle(
			fynetooltip.AddWindowToolTipLayer(u.SettingsUi.CanvasObject, u.Window.Canvas()),
			"User Settings",
		)
	})

	// testMenu := fyne.NewMenu("Test",
	// 	fyne.NewMenuItem("Add Item", func() { addItemWindow() }),
	// 	fyne.NewMenuItem("Set Items from JSON", func() {
	// 		dialog.ShowFileOpen(func(reader fyne.URIReadCloser, err error) {
	// 			log.Println(reader.URI().Path())
	// 			i := items.ParseItemsFromJson(reader.URI().Path())
	// 			for _, item := range i {
	// 				programs.CurrentProgram().Items[item.Name] = item
	// 			}
	// 			items.SetItemsMap(programs.CurrentProgram().Items)
	// 		}, u.win)
	// 	}),
	// 	fyne.NewMenuItem("Test string slice", func() {
	// 		log.Println("String Map:",
	// 			config.ViperConfig.Get("programs"+"."+config.DarkAndDarker+"."+"macros"),
	// 		)
	// 	}),
	// 	fyne.NewMenuItem("unregister failsafe", func() {
	// 		fs := []string{"esc", "ctrl", "shift"}

	// 		hook.Unregister(hook.KeyDown, fs)
	// 	}),
	// )

	// return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu)
	u.MainMenu.Items = append(u.MainMenu.Items, fyne.NewMenu("Settings", computerInfo, editor, userSettings), macroMenu)
	return u.MainMenu
}
