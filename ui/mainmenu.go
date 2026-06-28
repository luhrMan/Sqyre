package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/macro/actiondialog"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type actionTemplate struct {
	label      string
	actionType string
	category   string
	icon       fyne.Resource
	create     func() actions.ActionInterface
}

func buildActionTemplates() []actionTemplate {
	return []actionTemplate{
		{label: "Mouse Move", actionType: "move", category: "Mouse & Keyboard", icon: actions.NewMove("", false).Icon(), create: func() actions.ActionInterface {
			return actions.NewMove("", false)
		}},
		{label: "Click", actionType: "click", category: "Mouse & Keyboard", icon: actions.NewClick(false, true).Icon(), create: func() actions.ActionInterface { return actions.NewClick(false, true) }},
		{label: "Key", actionType: "key", category: "Mouse & Keyboard", icon: actions.NewKey("ctrl", true).Icon(), create: func() actions.ActionInterface { return actions.NewKey("ctrl", true) }},
		{label: "Type", actionType: "type", category: "Mouse & Keyboard", icon: actions.NewType("", 0).Icon(), create: func() actions.ActionInterface { return actions.NewType("", 0) }},
		{label: "Wait", actionType: "wait", category: "Miscellaneous", icon: actions.NewWait(0).Icon(), create: func() actions.ActionInterface { return actions.NewWait(0) }},
		{label: "Focus window", actionType: "focuswindow", category: "Miscellaneous", icon: actions.NewFocusWindow("", "").Icon(), create: func() actions.ActionInterface { return actions.NewFocusWindow("", "") }},
		{label: "Run macro", actionType: "runmacro", category: "Miscellaneous", icon: actions.NewRunMacro("").Icon(), create: func() actions.ActionInterface { return actions.NewRunMacro("") }},

		{label: "If", actionType: "conditional", category: "Miscellaneous", icon: actions.NewConditional(nil, actions.MatchAll, "", []actions.ActionInterface{}).Icon(), create: func() actions.ActionInterface {
			return actions.NewConditional(nil, actions.MatchAll, "", []actions.ActionInterface{})
		}},

		{label: "Loop", actionType: "loop", category: "Loop flow", icon: actions.NewLoop(1, "", []actions.ActionInterface{}).Icon(), create: func() actions.ActionInterface {
			return actions.NewLoop(1, "", []actions.ActionInterface{})
		}},
		{label: "Break", actionType: "break", category: "Loop flow", icon: actions.NewBreak().Icon(), create: func() actions.ActionInterface {
			return actions.NewBreak()
		}},
		{label: "Continue", actionType: "continue", category: "Loop flow", icon: actions.NewContinue().Icon(), create: func() actions.ActionInterface {
			return actions.NewContinue()
		}},
		{label: "Image Search", actionType: "imagesearch", category: "Detection", icon: actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, "", 1, 1, 0.95, 5).Icon(), create: func() actions.ActionInterface {
			return actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, "", 1, 1, 0.95, 5)
		}},
		{label: "OCR", actionType: "ocr", category: "Detection", icon: actions.NewOcr("", "template", "template search area").Icon(), create: func() actions.ActionInterface {
			return actions.NewOcr("", "template", "template search area")
		}},
		{label: "Find pixel", actionType: "findpixel", category: "Detection", icon: actions.NewFindPixel("", "", "ffffff", 0).Icon(), create: func() actions.ActionInterface {
			return actions.NewFindPixel("", "", "ffffff", 0)
		}},

		{label: "Set", actionType: "setvariable", category: "Variables", icon: actions.NewSetVariable("", "").Icon(), create: func() actions.ActionInterface { return actions.NewSetVariable("", "") }},
		{label: "Calculate", actionType: "calculate", category: "Variables", icon: actions.NewCalculate("", "").Icon(), create: func() actions.ActionInterface { return actions.NewCalculate("", "") }},
		{label: "For each row", actionType: "foreachrow", category: "Variables", icon: actions.NewForEachRow("", []actions.ListColumn{{}}, nil).Icon(), create: func() actions.ActionInterface {
			return actions.NewForEachRow("", []actions.ListColumn{{}}, nil)
		}},
		{label: "Save to", actionType: "savevariable", category: "Variables", icon: actions.NewSaveVariable("", "", false, false).Icon(), create: func() actions.ActionInterface {
			return actions.NewSaveVariable("", "", false, false)
		}},
	}
}

// AddActionPickerSize is the default size for the Add Action picker dialog and screenshots.
var AddActionPickerSize = fyne.NewSize(1240, 460)

func buildAddActionPickerContent(templates []actionTemplate, onPick func(actionTemplate)) fyne.CanvasObject {
	categoryColumns := []string{"Mouse & Keyboard", "Detection", "Variables", "Loop flow", "Miscellaneous"}
	categoryTiles := map[string][]fyne.CanvasObject{
		"Mouse & Keyboard": {},
		"Detection":        {},
		"Variables":        {},
		"Loop flow":        {},
		"Miscellaneous":    {},
	}
	for _, tmpl := range templates {
		t := tmpl
		bg := canvas.NewRectangle(actions.ActionPastelColor(t.actionType))
		bg.CornerRadius = 8
		bg.StrokeColor = theme.ShadowColor()
		bg.StrokeWidth = 1

		btn := widget.NewButtonWithIcon(t.label, t.icon, func() {
			if onPick != nil {
				onPick(t)
			}
		})
		btn.Importance = widget.LowImportance

		tile := container.NewStack(bg, container.NewPadded(btn))
		categoryTiles[t.category] = append(categoryTiles[t.category], container.NewPadded(tile))
	}

	columnObjects := make([]fyne.CanvasObject, 0, len(categoryColumns))
	for _, col := range categoryColumns {
		header := widget.NewLabel(col)
		header.TextStyle = fyne.TextStyle{Bold: true}
		content := append([]fyne.CanvasObject{header}, categoryTiles[col]...)
		columnObjects = append(columnObjects, container.NewVBox(content...))
	}

	grid := container.NewGridWithColumns(5, columnObjects...)
	return container.NewBorder(
		widget.NewLabel("Pick an action type"),
		nil, nil, nil,
		container.NewVScroll(grid),
	)
}

// AddActionPickerForScreenshot returns the Add Action picker grid for docs/tests.
func AddActionPickerForScreenshot() fyne.CanvasObject {
	return buildAddActionPickerContent(buildActionTemplates(), nil)
}

func showAddActionDialog(u *Ui, addActionAndRefresh func(actions.ActionInterface), templates []actionTemplate) {
	var d dialog.Dialog
	content := buildAddActionPickerContent(templates, func(t actionTemplate) {
		if d != nil {
			d.Hide()
		}
		addActionAndRefresh(t.create())
	})

	d = dialog.NewCustom("Add Action", "Close", content, u.Window)
	AddDialogEscapeClose(d, u.Window)
	d.Resize(AddActionPickerSize)
	d.Show()
}

func (u *Ui) constructMainMenu() *fyne.MainMenu {
	macroMenu := fyne.NewMenu("Macro")
	actionSubMenu := fyne.NewMenuItem("Add Blank Action", nil)
	actionPickerItem := fyne.NewMenuItem("Add Action...", nil)
	basicActionsSubMenu := fyne.NewMenuItem("Mouse & Keyboard", nil)
	advancedActionsSubMenu := fyne.NewMenuItem("Detection", nil)
	variableActionsSubMenu := fyne.NewMenuItem("Variables", nil)
	loopFlowActionsSubMenu := fyne.NewMenuItem("Loop flow", nil)
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
		loopFlowActionsSubMenu,
		miscActionsSubMenu,
	)
	addActionAndRefresh :=
		func(a actions.ActionInterface) {
			mt := u.Mui.MTabs.SelectedTab()
			if mt == nil {
				return
			}
			if !mt.InsertActionBelowSelection(a) {
				return
			}
			mt.Refresh()
			mt.Select(a.GetUID())
			mt.SelectedNode = a.GetUID()
			uid := a.GetUID()
			actiondialog.ShowActionDialog(a, func(updatedAction actions.ActionInterface) {
				if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
					log.Printf("failed to save macro after new action edit: %v", err)
				}
				mt.RefreshItem(uid)
				mt.Refresh()
			}, func() {
				mt.RecordMutation()
				if parent := a.GetParent(); parent != nil {
					parent.RemoveSubAction(a)
				}
				if mt.SelectedNode == uid {
					mt.SelectedNode = ""
				}
				mt.Refresh()
				if mt.OnTreeChanged != nil {
					mt.OnTreeChanged()
				}
			})
		}
	templates := buildActionTemplates()
	actionPickerItem.Action = func() {
		showAddActionDialog(u, addActionAndRefresh, templates)
	}

	basicItems := make([]*fyne.MenuItem, 0)
	advancedItems := make([]*fyne.MenuItem, 0)
	variableItems := make([]*fyne.MenuItem, 0)
	loopFlowItems := make([]*fyne.MenuItem, 0)
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
		case "Loop flow":
			loopFlowItems = append(loopFlowItems, item)
		case "Miscellaneous":
			miscItems = append(miscItems, item)
		}
	}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("", basicItems...)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("", advancedItems...)
	variableActionsSubMenu.ChildMenu = fyne.NewMenu("", variableItems...)
	loopFlowActionsSubMenu.ChildMenu = fyne.NewMenu("", loopFlowItems...)
	miscActionsSubMenu.ChildMenu = fyne.NewMenu("", miscItems...)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		if config.IsUITestMode() {
			str = "Total Screen Size: 1920x1080\nMonitor 1 Size: 1080x1920\n"
		} else {
			w, h := robotgo.GetScreenSize()
			str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
			for d := range robotgo.DisplaysNum() {
				_, _, mh, mw := robotgo.GetDisplayBounds(d)
				str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
			}
		}
		ShowInformationWithEscape("Computer Information", str, u.Window)
	})

	editor := fyne.NewMenuItem("Data Editor", func() {
		EnsureDataEditor()
		u.showOverlay(u.EditorUi.CanvasObject, "Editor", overlayEditor)
		if mt := GetUi().Mui.MTabs.SelectedTab(); mt != nil {
			mt.UnselectAll()
			mt.SelectedNode = ""
		}
	})

	userSettings := fyne.NewMenuItem("User Settings", func() {
		u.showOverlay(u.SettingsUi.CanvasObject, "User Settings", overlaySettings)
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
