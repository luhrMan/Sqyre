package binders

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"Sqyre/ui"
	"encoding/json"
	"errors"
	"log"
	"slices"
	"time"

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

	if names := getOpenMacroNames(); len(names) > 0 {
		for _, name := range names {
			m, err := repositories.MacroRepo().Get(name)
			if err != nil {
				log.Printf("Could not reopen macro %s: %v", name, err)
				continue
			}
			AddMacroTab(m)
		}
	} else {
		for _, m := range repositories.MacroRepo().GetAll() {
			AddMacroTab(m)
			break
		}
	}
	setMacroSelect(ui.GetUi().MainUi.Mui.MacroSelectButton)
	syncMacroToolbarFieldsFromSelection()
}

func SaveOpenMacros() {
	mtabs := ui.GetUi().Mui.MTabs
	var names []string
	for _, item := range mtabs.Items {
		names = append(names, item.Text)
	}
	data, err := json.Marshal(names)
	if err != nil {
		log.Printf("Error marshaling open macros: %v", err)
		return
	}
	fyne.CurrentApp().Preferences().SetString("OPEN_MACROS", string(data))
	log.Println("Saved open macros:", names)
}

func getOpenMacroNames() []string {
	s := fyne.CurrentApp().Preferences().String("OPEN_MACROS")
	if s == "" {
		return nil
	}
	var names []string
	if err := json.Unmarshal([]byte(s), &names); err != nil {
		log.Printf("Error unmarshaling open macros preference: %v", err)
		return nil
	}
	return names
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
	syncMacroToolbarFieldsFromSelection()
	services.RegisterMacroHotkey(m)
}

func syncMacroToolbarFieldsFromSelection() {
	mtabs := ui.GetUi().Mui.MTabs
	st := mtabs.SelectedTab()
	if st == nil || st.Macro == nil {
		return
	}
	mtabs.MacroNameEntry.SetText(st.Macro.Name)
	mtabs.BoundGlobalDelayEntry.SetValue(st.Macro.GlobalDelay)
	if len(st.Macro.Hotkey) == 0 {
		mtabs.MacroHotkeyLabel.SetText("—")
	} else {
		mtabs.MacroHotkeyLabel.SetText(services.ReverseParseMacroHotkey(st.Macro.Hotkey))
	}
	mtabs.HotkeyTriggerRadio.SetSelected(models.ParseHotkeyTrigger(st.Macro.HotkeyTrigger).UILabel())
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
			services.UnregisterMacroHotkey(m)
		}
		mtabs.SelectIndex(0)
		mtabs.BoundMacroListWidget.Refresh()
	}

	mtabs.OnUnselected = func(ti *container.TabItem) {
		mt := mtabs.SelectedTab()
		if mt != nil {
			mt.UnselectAll()
			mt.SelectedNode = ""
			RefreshItemsAccordionItems()
		}
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		if _, err := repositories.MacroRepo().Get(ti.Text); err != nil {
			log.Printf("Error getting macro %s: %v", ti.Text, err)
			return
		}

		syncMacroToolbarFieldsFromSelection()
	}

	saveHotkey := func(deferHookRegister bool) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		oldHk := slices.Clone(mt.Macro.Hotkey)
		oldTr := mt.Macro.HotkeyTrigger
		if !deferHookRegister {
			services.UnregisterHotkeyKeys(oldHk, oldTr)
		}

		disp := mtabs.MacroHotkeyLabel.Text
		if disp == "" || disp == "—" {
			mt.Macro.Hotkey = []string{}
		} else {
			mt.Macro.Hotkey = services.ParseMacroHotkey(disp)
		}
		mt.Macro.HotkeyTrigger = string(models.HotkeyTriggerFromUILabel(mtabs.HotkeyTriggerRadio.Selected))

		if !deferHookRegister {
			services.RegisterMacroHotkey(mt.Macro)
		}
		if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
			log.Printf("save hotkey: persist macro: %v", err)
		}
	}
	mtabs.MacroHotkeyRecordBtn.OnTapped = func() {
		ui.ShowHotkeyRecordDialog(ui.GetUi().Window, 1*time.Second, func(keys []string) {
			mtabs.MacroHotkeyLabel.SetText(services.ReverseParseMacroHotkey(keys))
			saveHotkey(true)
		})
	}
	mtabs.HotkeyTriggerRadio.OnChanged = func(string) {
		saveHotkey(false)
	}

	mtabs.MacroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			e := dialog.NewError(errors.New("macro name cannot be empty"), ui.GetUi().Window)
			ui.AddDialogEscapeClose(e, ui.GetUi().Window)
			e.Show()
			return
		}
		for _, m := range repositories.MacroRepo().GetAll() {
			if m.Name == sub {
				ui.ShowErrorWithEscape(errors.New("macro name already exists"), ui.GetUi().Window)
				return
			}
		}

		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}

		repositories.MacroRepo().Delete(mt.Macro.Name)

		mt.Macro.Name = sub
		mtabs.Selected().Text = sub

		repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro)

		mtabs.BoundMacroListWidget.Refresh()
		mtabs.Refresh()
	}
	mtabs.BoundGlobalDelayEntry.OnChanged = func(gd int) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		mt.Macro.GlobalDelay = gd
		robotgo.MouseSleep = gd
		robotgo.KeySleep = gd
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
					ui.ShowConfirmWithEscape("Delete Macro", "Are you sure you want to delete this macro?", func(b bool) {
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
	if mt == nil {
		return
	}
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		if st := ui.GetUi().Mui.MTabs.SelectedTab(); st != nil {
			st.SelectedNode = uid
		}
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
