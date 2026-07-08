package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/screen"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
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
		{label: "Mouse Move", actionType: "move", category: "Mouse & Keyboard", icon: actiondisplay.Icon(actions.NewMove("", true)), create: func() actions.ActionInterface {
			return actions.NewMove("", true)
		}},
		{label: "Click", actionType: "click", category: "Mouse & Keyboard", icon: actiondisplay.Icon(actions.NewClick(actions.ClickButtonLeft, true)), create: func() actions.ActionInterface { return actions.NewClick(actions.ClickButtonLeft, true) }},
		{label: "Key", actionType: "key", category: "Mouse & Keyboard", icon: actiondisplay.Icon(actions.NewKey("ctrl", true)), create: func() actions.ActionInterface { return actions.NewKey("ctrl", true) }},
		{label: "Type", actionType: "type", category: "Mouse & Keyboard", icon: actiondisplay.Icon(actions.NewType("", 0)), create: func() actions.ActionInterface { return actions.NewType("", 0) }},
		{label: "Wait", actionType: "wait", category: "Miscellaneous", icon: actiondisplay.Icon(actions.NewWait(0)), create: func() actions.ActionInterface { return actions.NewWait(0) }},
		{label: "Pause", actionType: "pause", category: "Miscellaneous", icon: actiondisplay.Icon(actions.NewPause("", nil, false)), create: func() actions.ActionInterface {
			return actions.NewPause("", nil, false)
		}},
		{label: "Focus window", actionType: "focuswindow", category: "Miscellaneous", icon: actiondisplay.Icon(actions.NewFocusWindow("", "")), create: func() actions.ActionInterface { return actions.NewFocusWindow("", "") }},
		{label: "Run macro", actionType: "runmacro", category: "Miscellaneous", icon: actiondisplay.Icon(actions.NewRunMacro("")), create: func() actions.ActionInterface { return actions.NewRunMacro("") }},

		{label: "If", actionType: "conditional", category: "Miscellaneous", icon: actiondisplay.Icon(actions.NewConditional(nil, actions.MatchAll, "", []actions.ActionInterface{})), create: func() actions.ActionInterface {
			return actions.NewConditional(nil, actions.MatchAll, "", []actions.ActionInterface{})
		}},

		{label: "Loop", actionType: "loop", category: "Loop flow", icon: actiondisplay.Icon(actions.NewLoop(1, "", []actions.ActionInterface{})), create: func() actions.ActionInterface {
			return actions.NewLoop(1, "", []actions.ActionInterface{})
		}},
		{label: "Break", actionType: "break", category: "Loop flow", icon: actiondisplay.Icon(actions.NewBreak()), create: func() actions.ActionInterface {
			return actions.NewBreak()
		}},
		{label: "Continue", actionType: "continue", category: "Loop flow", icon: actiondisplay.Icon(actions.NewContinue()), create: func() actions.ActionInterface {
			return actions.NewContinue()
		}},
		{label: "Image Search", actionType: "imagesearch", category: "Detection", icon: actiondisplay.Icon(actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, "", 1, 1, 0.95, 5)), create: func() actions.ActionInterface {
			return actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, "", 1, 1, 0.95, 5)
		}},
		{label: "OCR", actionType: "ocr", category: "Detection", icon: actiondisplay.Icon(actions.NewOcr("", "template", "template search area")), create: func() actions.ActionInterface {
			return actions.NewOcr("", "template", "template search area")
		}},
		{label: "Find pixel", actionType: "findpixel", category: "Detection", icon: actiondisplay.Icon(actions.NewFindPixel("", "", "ffffff", 0)), create: func() actions.ActionInterface {
			return actions.NewFindPixel("", "", "ffffff", 0)
		}},

		{label: "Set", actionType: "setvariable", category: "Variables", icon: actiondisplay.Icon(actions.NewSetVariable("", "")), create: func() actions.ActionInterface { return actions.NewSetVariable("", "") }},
		{label: "Calculate", actionType: "calculate", category: "Variables", icon: actiondisplay.Icon(actions.NewCalculate("", "")), create: func() actions.ActionInterface { return actions.NewCalculate("", "") }},
		{label: "For each row", actionType: "foreachrow", category: "Variables", icon: actiondisplay.Icon(actions.NewForEachRow("", []actions.ListColumn{{}}, nil)), create: func() actions.ActionInterface {
			return actions.NewForEachRow("", []actions.ListColumn{{}}, nil)
		}},
		{label: "Save to", actionType: "savevariable", category: "Variables", icon: actiondisplay.Icon(actions.NewSaveVariable("", "", false, false)), create: func() actions.ActionInterface {
			return actions.NewSaveVariable("", "", false, false)
		}},
	}
}

// AddActionPickerSize is the default size for the Add Action picker dialog and screenshots.
var AddActionPickerSize = fyne.NewSize(1500, 600)

func buildAddActionPickerContent(templates []actionTemplate, onPick func(actionTemplate)) fyne.CanvasObject {
	content, _ := buildAddActionPickerContentWithTiles(templates, onPick)
	return content
}

// buildAddActionPickerContentWithTiles builds the picker grid and returns each
// action's tile button keyed by label so docs frames can anchor a click guide
// on the real widget geometry instead of hardcoded coordinates.
func buildAddActionPickerContentWithTiles(templates []actionTemplate, onPick func(actionTemplate)) (fyne.CanvasObject, map[string]fyne.CanvasObject) {
	categoryColumns := []string{"Mouse & Keyboard", "Detection", "Variables", "Loop flow", "Miscellaneous"}
	categoryTiles := map[string][]fyne.CanvasObject{
		"Mouse & Keyboard": {},
		"Detection":        {},
		"Variables":        {},
		"Loop flow":        {},
		"Miscellaneous":    {},
	}
	tiles := make(map[string]fyne.CanvasObject, len(templates))
	for _, tmpl := range templates {
		t := tmpl
		bg := canvas.NewRectangle(actiondisplay.ActionPastelColorForApp(t.actionType))
		bg.CornerRadius = 8
		bg.StrokeColor = theme.Color(theme.ColorNameShadow)
		bg.StrokeWidth = 1

		btn := widget.NewButtonWithIcon(t.label, t.icon, func() {
			if onPick != nil {
				onPick(t)
			}
		})
		btn.Importance = widget.LowImportance
		tiles[t.label] = btn

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
	), tiles
}

// AddActionPickerForScreenshot returns the Add Action picker grid for docs/tests.
func AddActionPickerForScreenshot() fyne.CanvasObject {
	return buildAddActionPickerContent(buildActionTemplates(), nil)
}

// AddActionPickerWithTargetForScreenshot builds the picker and returns the tile
// button for the given action label so a demo frame can center its click guide
// on it.
func AddActionPickerWithTargetForScreenshot(label string) (fyne.CanvasObject, fyne.CanvasObject) {
	content, tiles := buildAddActionPickerContentWithTiles(buildActionTemplates(), nil)
	return content, tiles[label]
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

func (u *Ui) newAddActionAndRefresh() func(actions.ActionInterface) {
	return func(a actions.ActionInterface) {
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
		mt.EditAction(uid)
		if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
			log.Printf("failed to save macro after new action: %v", err)
		}
	}
}

// ShowAddActionPicker opens the new-action picker for the selected macro tab.
func (u *Ui) ShowAddActionPicker() {
	showAddActionDialog(u, u.newAddActionAndRefresh(), buildActionTemplates())
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
	addActionAndRefresh := u.newAddActionAndRefresh()
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
			w, h := screen.GetScreenSize()
			str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
			for d := 0; d < screen.NumDisplays(); d++ {
				_, _, mw, mh := screen.GetDisplayBounds(d)
				str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
			}
		}
		ShowInformationWithEscape("Computer Information", str, u.Window)
	})

	editor := fyne.NewMenuItem("Data Editor", func() {
		EnsureDataEditor()
		u.showOverlay(u.EditorUi.CanvasObject, "Editor", overlayEditor)
		if mt := u.Mui.MTabs.SelectedTab(); mt != nil {
			mt.UnselectAll()
			mt.SelectedNode = ""
		}
	})

	userSettings := fyne.NewMenuItem("User Settings", func() {
		u.showOverlay(u.SettingsUi.CanvasObject, "User Settings", overlaySettings)
	})

	u.MainMenu.Items = append(u.MainMenu.Items, fyne.NewMenu("Settings", computerInfo, editor, userSettings), macroMenu)
	return u.MainMenu
}
