package binders

import (
	"Squire/internal/assets"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/macro"
	"Squire/internal/utils"
	"Squire/ui"
	"errors"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type MacroBinding struct {
	*macro.Macro
	BoundSelectedAction binding.Struct
}

func BindMacros() {
	for _, macro := range GetMacros() {
		BindMacro(macro)
	}
}

func BindMacro(m *macro.Macro) {
	boundMacros[m.Name] = &MacroBinding{
		Macro: m,
		// BoundSelectedAction: binding.BindStruct(m.Root),
	}
}

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
	setMacroSelect()
	setMacroToolbar()
	for _, m := range GetMacros() {
		AddMacroTab(m)
	}

}

func AddMacroTab(m *macro.Macro) {
	mtabs := ui.GetUi().Mui.MTabs

	//check if already open. if it is, select it.
	t := container.NewTabItem(m.Name, ui.NewMacroTree(m))
	mtabs.AddTab(m.Name, t)
	setMacroTree(mtabs.SelectedTab())
	utils.RegisterHotkey(m.Hotkey, m.HotkeyCallback())
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

		BindMacro(macros[name])
		setMacroTree(ti.Content.(*ui.MacroTree))
		mtabs.BoundMacroListWidget.Refresh()
		return ti
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		utils.UnregisterHotkey(GetMacro(ti.Text).Hotkey)
		mtabs.BoundMacroListWidget.Refresh()
	}

	mtabs.OnUnselected = func(ti *container.TabItem) {
		ti.Content.(*ui.MacroTree).UnselectAll()
		UnbindAll()
		// boundMacros[ti.Content.(*ui.MacroTree).Macro.Name].BoundSelectedAction = nil
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		m := GetMacro(ti.Text)
		mtabs.MacroNameEntry.SetText(m.Name)
		mtabs.BoundGlobalDelayEntry.SetText(strconv.Itoa(m.GlobalDelay))

		mtabs.MacroHotkeyEntry.SetText(utils.ReverseParseMacroHotkey(m.Hotkey))
	}

	mtabs.MacroHotkeyEntry.PlaceHolder = "ctrl+shift+1 or ctrl+1 or ctrl+a+1"
	saveHotkey := func() {
		mt := mtabs.SelectedTab()
		utils.UnregisterHotkey(mt.Macro.Hotkey)
		mt.Macro.Hotkey = utils.ParseMacroHotkey(mtabs.MacroHotkeyEntry.Text)
		utils.RegisterHotkey(mt.Macro.Hotkey, mt.Macro.HotkeyCallback())
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
	ui.GetUi().Mui.MacroSelectButton = widget.NewButtonWithIcon("",
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
	// mt := ui.GetUi().Mui.MTabs.SelectedTab()
	at := ui.GetUi().ActionTabs
	macroBind := boundMacros[mt.Macro.Name]
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode = uid
		switch node := mt.Macro.Root.GetAction(uid).(type) {
		case *actions.Wait:
			macroBind.bindAction(node)
			at.SelectIndex(ui.WaitTab)
		case *actions.Move:
			macroBind.bindAction(node)
			at.SelectIndex(ui.MoveTab)
		case *actions.Click:
			macroBind.bindAction(node)
			at.SelectIndex(ui.ClickTab)
		case *actions.Key:
			macroBind.bindAction(node)
			at.SelectIndex(ui.KeyTab)

		case *actions.Loop:
			macroBind.bindAction(node)
			at.SelectIndex(ui.LoopTab)
		case *actions.ImageSearch:
			macroBind.bindAction(node)
			at.BoundTargetsGrid.Refresh()
			at.SelectIndex(ui.ImageSearchTab)
		case *actions.Ocr:
			macroBind.bindAction(node)
			at.SelectIndex(ui.OcrTab)
		}
	}
}

func setMacroToolbar() {
	ui.GetUi().Mui.MacroToolbars.TopToolbar.Objects[0].(*fyne.Container).Objects[0].(*widget.Toolbar).Prepend(widget.NewToolbarAction(theme.ContentAddIcon(), func() {
		var action actions.ActionInterface
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		selectedNode := mt.Macro.Root.GetAction(mt.SelectedNode)
		if selectedNode == nil {
			selectedNode = mt.Macro.Root
		}
		log.Println(ui.GetUi().ActionTabs.BoundMove)
		switch ui.ActionTabs.Selected(*ui.GetUi().ActionTabs).Text {
		case "Wait":
			time, e := ui.GetUi().ActionTabs.BoundWait.GetValue("Time")
			if e != nil {
				log.Println(e)
			}
			action = actions.NewWait(time.(int))
		case "Move":
			name, _ := ui.GetUi().ActionTabs.BoundMove.GetValue("Name")
			x, _ := ui.GetUi().ActionTabs.BoundMove.GetValue("X")
			y, _ := ui.GetUi().ActionTabs.BoundMove.GetValue("Y")
			action = actions.NewMove(coordinates.Point{Name: name.(string), X: x.(int), Y: y.(int)})
		case "Click":
			button, _ := ui.GetUi().ActionTabs.BoundClick.GetValue("Button")
			action = actions.NewClick(button.(string))
		case "Key":
			key, _ := ui.GetUi().ActionTabs.BoundKey.GetValue("Key")
			state, _ := ui.GetUi().ActionTabs.BoundKey.GetValue("State")
			action = actions.NewKey(key.(string), state.(string))
		case "Loop":
			name, _ := ui.GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
			count, _ := ui.GetUi().ActionTabs.BoundLoop.GetValue("Count")
			subactions := []actions.ActionInterface{}
			action = actions.NewLoop(count.(int), name.(string), subactions)
		case "Image":
			name, _ := ui.GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
			subactions := []actions.ActionInterface{}
			targets, _ := ui.GetUi().ActionTabs.BoundImageSearch.GetValue("Targets")
			searchArea, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("Name")
			x1, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("LeftX")
			y1, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("TopY")
			x2, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("RightX")
			y2, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("BottomY")
			action = actions.NewImageSearch(
				name.(string),
				subactions,
				targets.([]string),
				coordinates.SearchArea{Name: searchArea.(string), LeftX: x1.(int), TopY: y1.(int), RightX: x2.(int), BottomY: y2.(int)},
				// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
			)
		case "OCR":
			name, _ := ui.GetUi().ActionTabs.BoundAdvancedAction.GetValue("Name")
			target, _ := ui.GetUi().ActionTabs.BoundOcr.GetValue("Target")
			subactions := []actions.ActionInterface{}
			searchArea, _ := ui.GetUi().ActionTabs.BoundSearchArea.GetValue("Name")
			action = actions.NewOcr(
				name.(string),
				subactions,
				target.(string),
				searchArea.(coordinates.SearchArea),
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
