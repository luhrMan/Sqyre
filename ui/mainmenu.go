package ui

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"image/color"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
	"github.com/go-vgo/robotgo"
)

type actionTemplate struct {
	label    string
	category string
	icon     fyne.Resource
	create   func() actions.ActionInterface
}

func buildActionTemplates() []actionTemplate {
	return []actionTemplate{
		{label: "Mouse Move", category: "Mouse & Keyboard", icon: actions.NewMove(actions.Point{Name: "", X: 0, Y: 0}, false).Icon(), create: func() actions.ActionInterface {
			return actions.NewMove(actions.Point{Name: "", X: 0, Y: 0}, false)
		}},
		{label: "Click", category: "Mouse & Keyboard", icon: actions.NewClick(false, true).Icon(), create: func() actions.ActionInterface { return actions.NewClick(false, true) }},
		{label: "Key", category: "Mouse & Keyboard", icon: actions.NewKey("ctrl", true).Icon(), create: func() actions.ActionInterface { return actions.NewKey("ctrl", true) }},
		{label: "Type", category: "Mouse & Keyboard", icon: actions.NewType("", 0).Icon(), create: func() actions.ActionInterface { return actions.NewType("", 0) }},
		{label: "Wait", category: "Miscellaneous", icon: actions.NewWait(0).Icon(), create: func() actions.ActionInterface { return actions.NewWait(0) }},
		{label: "Focus window", category: "Miscellaneous", icon: actions.NewFocusWindow("").Icon(), create: func() actions.ActionInterface { return actions.NewFocusWindow("") }},
		{label: "Run macro", category: "Miscellaneous", icon: actions.NewRunMacro("").Icon(), create: func() actions.ActionInterface { return actions.NewRunMacro("") }},

		{label: "Loop", category: "Miscellaneous", icon: actions.NewLoop(1, "", []actions.ActionInterface{}).Icon(), create: func() actions.ActionInterface {
			return actions.NewLoop(1, "", []actions.ActionInterface{})
		}},
		{label: "Image Search", category: "Detection", icon: actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, actions.SearchArea{}, 1, 1, 0.95, 5).Icon(), create: func() actions.ActionInterface {
			return actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, actions.SearchArea{}, 1, 1, 0.95, 5)
		}},
		{label: "OCR", category: "Detection", icon: actions.NewOcr("", []actions.ActionInterface{}, "template", actions.SearchArea{Name: "template search area"}).Icon(), create: func() actions.ActionInterface {
			return actions.NewOcr("", []actions.ActionInterface{}, "template", actions.SearchArea{Name: "template search area"})
		}},
		{label: "Find pixel", category: "Detection", icon: actions.NewFindPixel("", actions.SearchArea{}, "ffffff", 0, nil).Icon(), create: func() actions.ActionInterface {
			return actions.NewFindPixel("", actions.SearchArea{}, "ffffff", 0, nil)
		}},

		{label: "Set", category: "Variables", icon: actions.NewSetVariable("", "").Icon(), create: func() actions.ActionInterface { return actions.NewSetVariable("", "") }},
		{label: "Calculate", category: "Variables", icon: actions.NewCalculate("", "").Icon(), create: func() actions.ActionInterface { return actions.NewCalculate("", "") }},
		{label: "Read from", category: "Variables", icon: actions.NewDataList("", "", false).Icon(), create: func() actions.ActionInterface { return actions.NewDataList("", "", false) }},
		{label: "Save to", category: "Variables", icon: actions.NewSaveVariable("", "", false, false).Icon(), create: func() actions.ActionInterface {
			return actions.NewSaveVariable("", "", false, false)
		}},
	}
}

func actionPastelColor(label, category string) color.Color {
	isDark := fyne.CurrentApp().Settings().ThemeVariant() == theme.VariantDark
	if isDark {
		if label == "Wait" {
			return color.NRGBA{R: 0x7B, G: 0x4E, B: 0x3E, A: 0xFF} // dark earth rust
		}
		switch category {
		case "Mouse & Keyboard":
			return color.NRGBA{R: 0x5E, G: 0x6B, B: 0x4A, A: 0xFF} // dark earth olive
		case "Detection":
			return color.NRGBA{R: 0x5A, G: 0x4A, B: 0x44, A: 0xFF} // dark earth umber
		case "Variables":
			return color.NRGBA{R: 0x7A, G: 0x63, B: 0x45, A: 0xFF} // dark earth tan
		case "Miscellaneous":
			return color.NRGBA{R: 0x6A, G: 0x5A, B: 0x3F, A: 0xFF} // dark earth ochre
		default:
			return color.NRGBA{R: 0x5C, G: 0x54, B: 0x49, A: 0xFF} // dark earth neutral
		}
	}

	// Light theme palette
	if label == "Wait" {
		return color.NRGBA{R: 0xC9, G: 0x8D, B: 0x6A, A: 0xFF} // earth terracotta
	}
	switch category {
	case "Mouse & Keyboard":
		return color.NRGBA{R: 0xA1, G: 0xB0, B: 0x7A, A: 0xFF} // earth sage
	case "Detection":
		return color.NRGBA{R: 0xB4, G: 0x9A, B: 0x84, A: 0xFF} // earth clay
	case "Variables":
		return color.NRGBA{R: 0xC7, G: 0xAE, B: 0x7B, A: 0xFF} // earth sand
	case "Miscellaneous":
		return color.NRGBA{R: 0xB8, G: 0x9A, B: 0x6A, A: 0xFF} // earth ochre
	default:
		return color.NRGBA{R: 0xB2, G: 0xA4, B: 0x8E, A: 0xFF} // earth neutral
	}
}

func showAddActionDialog(u *Ui, addActionAndRefresh func(actions.ActionInterface), templates []actionTemplate) {
	var d dialog.Dialog
	categoryColumns := []string{"Mouse & Keyboard", "Detection", "Variables", "Miscellaneous"}
	categoryTiles := map[string][]fyne.CanvasObject{
		"Mouse & Keyboard": {},
		"Detection":        {},
		"Variables":        {},
		"Miscellaneous":    {},
	}
	for _, tmpl := range templates {
		t := tmpl
		bg := canvas.NewRectangle(actionPastelColor(t.label, t.category))
		bg.CornerRadius = 8
		bg.StrokeColor = theme.ShadowColor()
		bg.StrokeWidth = 1

		btn := widget.NewButtonWithIcon(t.label, t.icon, func() {
			if d != nil {
				d.Hide()
			}
			addActionAndRefresh(t.create())
		})
		btn.Importance = widget.LowImportance

		tile := container.NewStack(bg, container.NewPadded(btn))
		categoryTiles[t.category] = append(categoryTiles[t.category], tile)
	}

	columnObjects := make([]fyne.CanvasObject, 0, len(categoryColumns))
	for _, col := range categoryColumns {
		header := widget.NewLabel(col)
		header.TextStyle = fyne.TextStyle{Bold: true}
		content := append([]fyne.CanvasObject{header}, categoryTiles[col]...)
		columnObjects = append(columnObjects, container.NewVBox(content...))
	}

	grid := container.NewGridWithColumns(4, columnObjects...)
	content := container.NewBorder(
		widget.NewLabel("Pick an action type"),
		nil, nil, nil,
		container.NewVScroll(grid),
	)

	d = dialog.NewCustom("Add Action", "Close", content, u.Window)
	AddDialogEscapeClose(d, u.Window)
	d.Resize(fyne.NewSize(980, 460))
	d.Show()
}

func (u *Ui) constructMainMenu() *fyne.MainMenu {
	macroMenu := fyne.NewMenu("Macro")
	actionSubMenu := fyne.NewMenuItem("Add Blank Action", nil)
	actionPickerItem := fyne.NewMenuItem("Add Action...", nil)
	basicActionsSubMenu := fyne.NewMenuItem("Mouse & Keyboard", nil)
	advancedActionsSubMenu := fyne.NewMenuItem("Detection", nil)
	variableActionsSubMenu := fyne.NewMenuItem("Variables", nil)
	miscActionsSubMenu := fyne.NewMenuItem("Miscellaneous", nil)

	macroMenu.Items = append(macroMenu.Items,
		actionPickerItem,
		actionSubMenu,
	)
	actionSubMenu.ChildMenu = fyne.NewMenu("")

	actionSubMenu.ChildMenu.Items = append(actionSubMenu.ChildMenu.Items,
		basicActionsSubMenu,
		advancedActionsSubMenu,
		variableActionsSubMenu,
		miscActionsSubMenu,
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
			uid := a.GetUID()
			ShowActionDialog(a, func(updatedAction actions.ActionInterface) {
				if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
					log.Printf("failed to save macro after new action edit: %v", err)
				}
				mt.RefreshItem(uid)
				mt.Refresh()
			})
		}
	templates := buildActionTemplates()
	actionPickerItem.Action = func() {
		showAddActionDialog(u, addActionAndRefresh, templates)
	}

	basicItems := make([]*fyne.MenuItem, 0)
	advancedItems := make([]*fyne.MenuItem, 0)
	variableItems := make([]*fyne.MenuItem, 0)
	miscItems := make([]*fyne.MenuItem, 0)
	for _, tmpl := range templates {
		t := tmpl
		item := fyne.NewMenuItemWithIcon(t.label, t.icon, func() {
			addActionAndRefresh(t.create())
		})
		switch t.category {
		case "Mouse & Keyboard":
			basicItems = append(basicItems, item)
		case "Detection":
			advancedItems = append(advancedItems, item)
		case "Variables":
			variableItems = append(variableItems, item)
		case "Miscellaneous":
			miscItems = append(miscItems, item)
		}
	}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("", basicItems...)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("", advancedItems...)
	variableActionsSubMenu.ChildMenu = fyne.NewMenu("", variableItems...)
	miscActionsSubMenu.ChildMenu = fyne.NewMenu("", miscItems...)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		w, h := robotgo.GetScreenSize()
		str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
		for d := range robotgo.DisplaysNum() {
			_, _, mh, mw := robotgo.GetDisplayBounds(d)
			str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
		}
		ShowInformationWithEscape("Computer Information", str, u.Window)
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
