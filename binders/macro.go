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

func SetMacroUi() {

	setMtabSettingsAndWidgets()

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
		ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode = uid
	}
	mt.OnOpenActionDialog = func(action actions.ActionInterface) {
		if action == nil {
			return
		}
		uid := action.GetUID()
		ui.ShowActionDialog(action, func(updatedAction actions.ActionInterface) {
			if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
				log.Printf("failed to save macro after action edit: %v", err)
			}
			mt.RefreshItem(uid)
			mt.Refresh()
		})
	}
}
