package binders

import (
	"Squire/internal/assets"
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/macro"
	"Squire/internal/services"
	"Squire/ui"
	"errors"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

// type BoundMacros struct {
// 	binds []binding.Struct
// }

// func BindMacros() {
// 	for _, macro := range GetMacros() {
// 		BindMacro(macro)
// 	}
// }

// func BindMacro(m *macro.Macro) {
// 	boundMacros.binds = append(boundMacros.binds, binding.BindStruct(m))
// }

func GetMacros() map[string]*macro.Macro {
	return macros
}

func GetMacro(s string) *macro.Macro {
	for _, m := range GetMacros() {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func AddMacro(s string, d int) {
	if s == "" {
		return
	}
	macros[s] = macro.NewMacro(s, d, []string{})
}

func GetMacrosAsStringSlice() []string {
	keys := make([]string, len(GetMacros()))

	i := 0
	for _, k := range GetMacros() {
		keys[i] = k.Name
		i++
	}
	return keys
}

func SetMacroUi() {
	setMtabSettingsAndWidgets()
	setMacroToolbar()
	setMacroSelect()
	for _, m := range GetMacros() {
		AddMacroTab(m)
	}

}

func AddMacroTab(m *macro.Macro) {
	mtabs := ui.GetUi().Mui.MTabs
	t := container.NewTabItem(m.Name, ui.NewMacroTree(m))
	mtabs.AddTab(m.Name, t)
	setMacroTree(mtabs.SelectedTab())
	services.RegisterHotkey(m.Hotkey, services.MacroHotkeyCallback(m))
}

func setMtabSettingsAndWidgets() {
	mtabs := ui.GetUi().Mui.MTabs
	mtabs.CreateTab = func() *container.TabItem {
		macros := GetMacros()

		c := len(macros)
		name := "New macro " + strconv.Itoa(c)
		for {
			if _, ok := macros[name]; ok {
				c++
				name = "New macro " + strconv.Itoa(c)
			} else {
				break
			}
		}

		// for {
		// 	if slices.Contains(macros, GetMacro(name)) {
		// 		c++
		// 		name = "New macro " + strconv.Itoa(c)
		// 	} else {
		// 		break
		// 	}
		// }
		AddMacro(name, 0)
		ti := container.NewTabItem(
			name,
			ui.NewMacroTree(GetMacro(name)),
		)

		setMacroTree(ti.Content.(*ui.MacroTree))
		mtabs.BoundMacroListWidget.Refresh()
		return ti
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		services.UnregisterHotkey(GetMacro(ti.Text).Hotkey)
		mtabs.BoundMacroListWidget.Refresh()
	}

	mtabs.OnUnselected = func(ti *container.TabItem) {
		mt := mtabs.SelectedTab()
		mt.UnselectAll()
		mt.SelectedNode = ""
		ResetBinds()
		RefreshItemsAccordionItems()
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		m := GetMacro(ti.Text)
		mtabs.MacroNameEntry.SetText(m.Name)
		mtabs.BoundGlobalDelayEntry.SetText(strconv.Itoa(m.GlobalDelay))

		mtabs.MacroHotkeyEntry.SetText(services.ReverseParseMacroHotkey(m.Hotkey))
	}

	mtabs.MacroHotkeyEntry.PlaceHolder = "ctrl+shift+1 or ctrl+1 or ctrl+a+1"
	saveHotkey := func() {
		mt := mtabs.SelectedTab()
		m := mt.Macro
		services.UnregisterHotkey(mt.Macro.Hotkey)
		m.Hotkey = services.ParseMacroHotkey(mtabs.MacroHotkeyEntry.Text)
		services.RegisterHotkey(mt.Macro.Hotkey, services.MacroHotkeyCallback(m))
	}
	mtabs.MacroHotkeyEntry.ActionItem = widget.NewButtonWithIcon("", theme.DocumentSaveIcon(), func() {
		saveHotkey()
	})
	mtabs.MacroHotkeyEntry.OnSubmitted = func(s string) {
		saveHotkey()
	}

	mtabs.MacroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			e := dialog.NewError(errors.New("macro name cannot be empty"), ui.GetUi().MainWindow)
			e.Show()
			return
		}
		for _, m := range GetMacros() {
			if m.Name == sub {
				dialog.ShowError(errors.New("macro name already exists"), ui.GetUi().MainWindow)
				return
			}
		}
		mt := mtabs.SelectedTab()
		mt.Macro.Name = sub
		mtabs.Selected().Text = sub
		mtabs.BoundMacroListWidget.Refresh()
		mtabs.Refresh()
	}
	mtabs.BoundGlobalDelayEntry.OnChanged = func(s string) {
		mt := mtabs.SelectedTab()
		gd, _ := strconv.Atoi(s)

		mt.Macro.GlobalDelay = gd
		robotgo.MouseSleep = gd
		robotgo.KeySleep = gd
	}
}

func setMacroSelect() {
	ui.GetUi().Mui.MacroSelectButton = widget.NewButtonWithIcon("why no show",
		theme.FolderOpenIcon(),
		func() {
			title := "Open Macro"
			for _, w := range fyne.CurrentApp().Driver().AllWindows() {
				if w.Title() == title {
					w.RequestFocus()
					return
				}
			}
			w := fyne.CurrentApp().NewWindow(title)
			w.SetIcon(assets.AppIcon)
			ui.GetUi().Mui.MTabs.BoundMacroListWidget = widget.NewList(
				func() int {
					return len(GetMacros())
				},
				func() fyne.CanvasObject {
					return widget.NewLabel("template")
				},
				func(id widget.ListItemID, co fyne.CanvasObject) {
					k := GetMacrosAsStringSlice()
					label := co.(*widget.Label)
					slices.Sort(k)
					v := k[id]
					label.SetText(v)
					label.Importance = widget.MediumImportance
					for _, d := range ui.GetUi().Mui.MTabs.Items {
						if d.Text == v {
							label.Importance = widget.SuccessImportance
						}
					}
					label.Refresh()
				},
			)
			ui.GetUi().Mui.MTabs.BoundMacroListWidget.OnSelected =
				func(id widget.ListItemID) {
					k := GetMacrosAsStringSlice()
					slices.Sort(k)
					AddMacroTab(GetMacro(k[id]))
					ui.GetUi().Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
					ui.GetUi().Mui.MTabs.BoundMacroListWidget.UnselectAll()
				}
			w.SetContent(
				container.NewAdaptiveGrid(1,
					ui.GetUi().Mui.MTabs.BoundMacroListWidget,
				),
			)
			w.Resize(fyne.NewSize(300, 500))
			w.Show()
		},
	)
}

// MACRO TREE
func setMacroTree(mt *ui.MacroTree) {
	ats := ui.GetUi().ActionTabs
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode = uid
		switch node := mt.Macro.Root.GetAction(uid).(type) {
		case *actions.Wait:
			bindAction(node)
			ats.SelectIndex(ui.WaitTab)
		case *actions.Move:
			bindAction(node)
			ats.SelectIndex(ui.MoveTab)
		case *actions.Click:
			bindAction(node)
			ats.SelectIndex(ui.ClickTab)
		case *actions.Key:
			bindAction(node)
			ats.SelectIndex(ui.KeyTab)

		case *actions.Loop:
			bindAction(node)
			ats.SelectIndex(ui.LoopTab)
		case *actions.ImageSearch:
			bindAction(node)
			ats.SelectIndex(ui.ImageSearchTab)
		case *actions.Ocr:
			bindAction(node)
			ats.SelectIndex(ui.OcrTab)
		}
	}
	mt.Tree.OnUnselected = func(uid widget.TreeNodeID) {
		ResetBinds()
	}
}

func setMacroToolbar() {
	ui.GetUi().Mui.MacroToolbars.TopToolbar.Objects[0].(*fyne.Container).Objects[0].(*widget.Toolbar).Prepend(widget.NewToolbarAction(theme.ContentAddIcon(), func() {
		var action actions.ActionInterface
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		ats := ui.GetUi().ActionTabs
		selectedNode := mt.Macro.Root.GetAction(mt.SelectedNode)
		if selectedNode == nil {
			selectedNode = mt.Macro.Root
		}
		switch ui.ActionTabs.Selected(*ats).Text {
		case "Wait":
			time, e := ats.BoundWait.GetValue("Time")
			if e != nil {
				log.Println(e)
			}
			action = actions.NewWait(time.(int))
		case "Move":
			name, _ := ats.BoundPoint.GetValue("Name")
			x, _ := ats.BoundPoint.GetValue("X")
			y, _ := ats.BoundPoint.GetValue("Y")
			action = actions.NewMove(coordinates.Point{Name: name.(string), X: x.(int), Y: y.(int)})
		case "Click":
			button, _ := ats.BoundClick.GetValue("Button")
			action = actions.NewClick(button.(bool))
		case "Key":
			key, _ := ats.BoundKey.GetValue("Key")
			state, _ := ats.BoundKey.GetValue("State")
			action = actions.NewKey(key.(string), state.(bool))
		case "Loop":
			name, _ := ats.BoundLoopAA.GetValue("Name")
			count, _ := ats.BoundLoop.GetValue("Count")
			subactions := []actions.ActionInterface{}
			action = actions.NewLoop(count.(int), name.(string), subactions)
		case "Image":
			name, _ := ats.BoundImageSearchAA.GetValue("Name")
			subactions := []actions.ActionInterface{}
			targets, _ := ats.BoundImageSearch.GetValue("Targets")
			rs, _ := ats.BoundImageSearch.GetValue("RowSplit")
			cs, _ := ats.BoundImageSearch.GetValue("ColSplit")
			tol, _ := ats.BoundImageSearch.GetValue("Tolerance")
			searchArea, _ := ats.BoundImageSearchSA.GetValue("Name")
			x1, _ := ats.BoundImageSearchSA.GetValue("LeftX")
			y1, _ := ats.BoundImageSearchSA.GetValue("TopY")
			x2, _ := ats.BoundImageSearchSA.GetValue("RightX")
			y2, _ := ats.BoundImageSearchSA.GetValue("BottomY")
			action = actions.NewImageSearch(
				name.(string),
				subactions,
				targets.([]string),
				coordinates.SearchArea{Name: searchArea.(string), LeftX: x1.(int), TopY: y1.(int), RightX: x2.(int), BottomY: y2.(int)},
				rs.(int), cs.(int), tol.(float32),
				// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
			)
		case "OCR":
			name, _ := ats.BoundOcrAA.GetValue("Name")
			target, _ := ats.BoundOcr.GetValue("Target")
			subactions := []actions.ActionInterface{}
			searchArea, _ := ats.BoundOcrSA.GetValue("Name")
			x1, _ := ats.BoundOcrSA.GetValue("LeftX")
			y1, _ := ats.BoundOcrSA.GetValue("TopY")
			x2, _ := ats.BoundOcrSA.GetValue("RightX")
			y2, _ := ats.BoundOcrSA.GetValue("BottomY")
			action = actions.NewOcr(
				name.(string),
				subactions,
				target.(string),
				coordinates.SearchArea{Name: searchArea.(string), LeftX: x1.(int), TopY: y1.(int), RightX: x2.(int), BottomY: y2.(int)},
				// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
			)
		}

		// if selectedNode == nil {
		// 	selectedNode = mt.Macro.Root
		// }
		if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
			s.AddSubAction(action)
		} else {
			selectedNode.GetParent().AddSubAction(action)
		}
		mt.Select(action.GetUID())
		mt.RefreshItem(action.GetUID())
	}))

}
