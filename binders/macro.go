package binders

import (
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/internal/services"
	"Squire/ui"
	"errors"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	"github.com/google/uuid"
)

// func AddMacro(s string, d int) {
// 	if s == "" {
// 		return
// 	}
// 	macros[s] = macro.NewMacro(s, d, []string{})
// }

func SetMacroUi() {
	// mtabs := ui.GetUi().Mui.MTabs
	// mtabs.OnSelected = func(ti *container.TabItem) {
	// 	setMtabSettingsAndWidgets()

	// 	m := repositories.MacroRepo().Get(ti.Text)
	// 	mtabs.MacroNameEntry.SetText(m.Name)
	// 	mtabs.BoundGlobalDelayEntry.SetText(strconv.Itoa(m.GlobalDelay))

	// 	mtabs.MacroHotkeyEntry.SetText(services.ReverseParseMacroHotkey(m.Hotkey))
	// }
	setMtabSettingsAndWidgets()

	// setMacroToolbar()
	for _, m := range repositories.MacroRepo().GetAll() {
		AddMacroTab(m)
	}
	setMacroSelect(ui.GetUi().MainUi.Mui.MacroSelectButton)
}

func AddMacroTab(m *models.Macro) {
	mtabs := ui.GetUi().Mui.MTabs
	for _, d := range mtabs.Items {
		if d.Text == m.Name {
			log.Println("macro already open")
			mtabs.Select(d)
			return
		}
	}
	t := container.NewTabItem(m.Name, ui.NewMacroTree(m))
	mtabs.AddTab(m.Name, t)
	setMacroTree(mtabs.SelectedTab())
	services.RegisterHotkey(m.Hotkey, services.MacroHotkeyCallback(m))
}

func setMtabSettingsAndWidgets() {
	mtabs := ui.GetUi().Mui.MTabs
	mtabs.CreateTab = func() *container.TabItem {
		name := "new macro " + uuid.NewString()
		m := models.NewMacro(name, 0, []string{})
		repositories.MacroRepo().Set(m.Name, m)
		// m, err := repositories.MacroRepo().Get(name)
		// if err != nil {
		// 	log.Println("Error creating macro tab")
		// 	return nil
		// }
		ti := container.NewTabItem(
			name,
			ui.NewMacroTree(m),
		)

		setMacroTree(ti.Content.(*ui.MacroTree))
		go fyne.DoAndWait(func() {
			mtabs.BoundMacroListWidget.Refresh()
		})
		return ti
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		m, err := repositories.MacroRepo().Get(ti.Text)
		if err == nil {
			services.UnregisterHotkey(m.Hotkey)
		}
		mtabs.SelectIndex(0)
		mtabs.BoundMacroListWidget.Refresh()
	}

	mtabs.OnUnselected = func(ti *container.TabItem) {
		mt := mtabs.SelectedTab()
		mt.UnselectAll()
		mt.SelectedNode = ""
		// ResetBinds()
		RefreshItemsAccordionItems()
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		m, err := repositories.MacroRepo().Get(ti.Text)
		if err != nil {
			log.Printf("Error getting macro %s: %v", ti.Text, err)
			return
		}

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
			e := dialog.NewError(errors.New("macro name cannot be empty"), ui.GetUi().Window)
			e.Show()
			return
		}
		for _, m := range repositories.MacroRepo().GetAll() {
			if m.Name == sub {
				dialog.ShowError(errors.New("macro name already exists"), ui.GetUi().Window)
				return
			}
		}

		mt := mtabs.SelectedTab()

		repositories.MacroRepo().Delete(mt.Macro.Name)

		mt.Macro.Name = sub
		mtabs.Selected().Text = sub

		repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro)

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
	mtabs.BoundGlobalDelayEntry.OnSubmitted = func(s string) {
		mt := mtabs.SelectedTab()
		repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro)
	}
}

func setMacroSelect(b *widget.Button) {
	var popup *widget.PopUp
	b.Text = ""
	b.Icon = theme.ListIcon()
	b.OnTapped = func() {
		ui.GetUi().Mui.MTabs.BoundMacroListWidget = widget.NewList(
			func() int {
				return len(repositories.MacroRepo().GetAll())
			},
			func() fyne.CanvasObject {
				return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance})
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				k := repositories.MacroRepo().GetAllKeys()
				c := co.(*fyne.Container)
				label := c.Objects[0].(*widget.Label)
				removeButton := c.Objects[2].(*widget.Button)
				// label := co.(*widget.Label)
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
				removeButton.OnTapped = func() {
					m, err := repositories.MacroRepo().Get(v)
					if err != nil {
						log.Printf("Error getting macro %s: %v", v, err)
						return
					}
					dialog.ShowConfirm("Delete Macro", "Are you sure you want to delete this macro?", func(b bool) {
						if b {
							if err := repositories.MacroRepo().Delete(v); err != nil {
								log.Printf("Error deleting macro %s: %v", v, err)
								return
							}
							ui.GetUi().Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
							for _, ti := range ui.GetUi().Mui.MTabs.Items {
								if m.Name == ti.Text {
									ui.GetUi().Mui.MTabs.Remove(ti)
								}
							}
							ui.GetUi().Mui.MTabs.Refresh()
						}
					}, ui.GetUi().Window)
				}
				removeButton.Show()

			},
		)
		ui.GetUi().Mui.MTabs.BoundMacroListWidget.OnSelected =
			func(id widget.ListItemID) {
				k := repositories.MacroRepo().GetAllKeys()
				slices.Sort(k)
				log.Println(k[id])
				m, err := repositories.MacroRepo().Get(k[id])
				if err != nil {
					log.Printf("Error getting macro %s: %v", k[id], err)
					return
				}
				AddMacroTab(m)
				ui.GetUi().Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
				ui.GetUi().Mui.MTabs.BoundMacroListWidget.UnselectAll()
			}
		popUpContent := container.NewBorder(
			widget.NewButton("Close", func() {
				popup.Hide() // Function to hide the pop-up
			}), nil, nil, nil,
			container.NewAdaptiveGrid(1,
				ui.GetUi().Mui.MTabs.BoundMacroListWidget,
			),
		)
		popup = widget.NewModalPopUp(popUpContent, ui.GetUi().Window.Canvas())
		popup.Resize(fyne.NewSize(300, 500))
		popup.Show()
	}
}

// MACRO TREE
func setMacroTree(mt *ui.MacroTree) {
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		// Update selected node
		ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode = uid
		action := mt.Macro.Root.GetAction(uid)
		if action != nil {
			// Show dialog for editing the action (even if already selected)
			ui.ShowActionDialog(action, func(updatedAction actions.ActionInterface) {
				// Refresh the tree after saving
				mt.RefreshItem(uid)
				mt.Refresh()
			})
		}
	}
	// mt.Tree.OnUnselected = func(uid widget.TreeNodeID) {
	// 	ResetBinds()
	// }
}

// func setMacroToolbar() {
// 	// ui.GetUi().Mui.MacroToolbars.TopToolbar.Objects[0].(*fyne.Container).Objects[0].(*widget.Toolbar).Prepend(widget.NewToolbarAction(theme.ContentAddIcon(), func() {
// 	ui.GetUi().Mui.MacroToolbars.TopToolbar.Objects[0].(*fyne.Container).Objects[0].(*ttwidget.Button).OnTapped = func() {
// 		var action actions.ActionInterface
// 		mt := ui.GetUi().Mui.MTabs.SelectedTab()
// 		// ats := ui.GetUi().ActionTabs
// 		// selectedNode := mt.Macro.Root.GetAction(mt.SelectedNode)
// 		// if selectedNode == nil {
// 		// 	selectedNode = mt.Macro.Root
// 		// }
// 		// switch ui.ActionTabs.Selected(*ats).Text {
// 		// case "Wait":
// 		// 	time, e := ats.BoundWait.GetValue("Time")
// 		// 	if e != nil {
// 		// 		log.Println(e)
// 		// 	}
// 		// 	action = actions.NewWait(time.(int))
// 		// case "Move":
// 		// 	name, _ := ats.BoundPoint.GetValue("Name")
// 		// 	x, _ := ats.BoundPoint.GetValue("X")
// 		// 	y, _ := ats.BoundPoint.GetValue("Y")
// 		// 	action = actions.NewMove(actions.Point{Name: name.(string), X: x.(int), Y: y.(int)})
// 		// case "Click":
// 		// 	button, _ := ats.BoundClick.GetValue("Button")
// 		// 	action = actions.NewClick(button.(bool))
// 		// case "Key":
// 		// 	key, _ := ats.BoundKey.GetValue("Key")
// 		// 	state, _ := ats.BoundKey.GetValue("State")
// 		// 	action = actions.NewKey(key.(string), state.(bool))
// 		// case "Loop":
// 		// 	name, _ := ats.BoundLoopAA.GetValue("Name")
// 		// 	count, _ := ats.BoundLoop.GetValue("Count")
// 		// 	subactions := []actions.ActionInterface{}
// 		// 	action = actions.NewLoop(count.(int), name.(string), subactions)
// 		// case "Image":
// 		// 	name, _ := ats.BoundImageSearchAA.GetValue("Name")
// 		// 	subactions := []actions.ActionInterface{}
// 		// 	targets, _ := ats.BoundImageSearch.GetValue("Targets")
// 		// 	rs, _ := ats.BoundImageSearch.GetValue("RowSplit")
// 		// 	cs, _ := ats.BoundImageSearch.GetValue("ColSplit")
// 		// 	tol, _ := ats.BoundImageSearch.GetValue("Tolerance")
// 		// 	searchArea, _ := ats.BoundImageSearchSA.GetValue("Name")
// 		// 	x1, _ := ats.BoundImageSearchSA.GetValue("LeftX")
// 		// 	y1, _ := ats.BoundImageSearchSA.GetValue("TopY")
// 		// 	x2, _ := ats.BoundImageSearchSA.GetValue("RightX")
// 		// 	y2, _ := ats.BoundImageSearchSA.GetValue("BottomY")
// 		// 	action = actions.NewImageSearch(
// 		// 		name.(string),
// 		// 		subactions,
// 		// 		targets.([]string),
// 		// 		actions.SearchArea{Name: searchArea.(string), LeftX: x1.(int), TopY: y1.(int), RightX: x2.(int), BottomY: y2.(int)},
// 		// 		rs.(int), cs.(int), tol.(float32),
// 		// 		// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
// 		// 	)
// 		// case "OCR":
// 		// 	name, _ := ats.BoundOcrAA.GetValue("Name")
// 		// 	target, _ := ats.BoundOcr.GetValue("Target")
// 		// 	subactions := []actions.ActionInterface{}
// 		// 	searchArea, _ := ats.BoundOcrSA.GetValue("Name")
// 		// 	x1, _ := ats.BoundOcrSA.GetValue("LeftX")
// 		// 	y1, _ := ats.BoundOcrSA.GetValue("TopY")
// 		// 	x2, _ := ats.BoundOcrSA.GetValue("RightX")
// 		// 	y2, _ := ats.BoundOcrSA.GetValue("BottomY")
// 		// 	action = actions.NewOcr(
// 		// 		name.(string),
// 		// 		subactions,
// 		// 		target.(string),
// 		// 		actions.SearchArea{Name: searchArea.(string), LeftX: x1.(int), TopY: y1.(int), RightX: x2.(int), BottomY: y2.(int)},
// 		// 		// binders.GetProgram(config.DarkAndDarker).Coordinates[config.MainMonitorSizeString].GetSearchArea(searchArea.(string))
// 		// 	)
// 		// }

// 		// if selectedNode == nil {
// 		// 	selectedNode = mt.Macro.Root
// 		// }
// 		// if s, ok := selectedNode.(actions.AdvancedActionInterface); ok {
// 		// 	s.AddSubAction(action)
// 		// } else {
// 		// 	selectedNode.GetParent().AddSubAction(action)
// 		// }
// 		mt.Select(action.GetUID())
// 		mt.RefreshItem(action.GetUID())
// 	}

// }
