package macro

import (
	"Sqyre/internal/appdata"
	"Sqyre/internal/fyneui"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
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
	"github.com/google/uuid"
)

// WireDeps supplies window, macro UI shell, and dialog callbacks from package ui (avoids import cycle).
type WireDeps struct {
	Window                fyne.Window
	Mui                   *MacroUi
	RefreshItemsAccordion func()
	ShowHotkeyRecordDialog func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string))
	ShowErrorWithEscape    func(err error, parent fyne.Window)
	AddDialogEscapeClose   func(d dialog.Dialog, parent fyne.Window)
	ShowConfirmWithEscape  func(title, message string, callback func(bool), parent fyne.Window)
	ShowActionDialog       func(action actions.ActionInterface, onSave func(actions.ActionInterface))
}

var activeWire WireDeps

// SetMacroUi wires macro tabs, toolbar handlers, and restores open macros from preferences.
func SetMacroUi(d WireDeps) {
	activeWire = d
	setMtabSettingsAndWidgets(d)

	if names := getOpenMacroNames(); len(names) > 0 {
		for _, name := range names {
			m, err := appdata.Macros().Get(name)
			if err != nil {
				log.Printf("Could not reopen macro %s: %v", name, err)
				continue
			}
			AddMacroTab(m)
		}
	} else {
		for _, m := range appdata.Macros().GetAll() {
			AddMacroTab(m)
			break
		}
	}
	setMacroSelect(d.Mui.MacroSelectButton, d)
	syncMacroToolbarFieldsFromSelection()
}

// SaveOpenMacros persists which macro tabs are open.
func SaveOpenMacros() {
	mtabs := activeWire.Mui.MTabs
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

// AddMacroTab opens a macro in a new tab (or selects it if already open).
func AddMacroTab(m *models.Macro) {
	mtabs := activeWire.Mui.MTabs
	for _, d := range mtabs.Items {
		if d.Text == m.Name {
			log.Println("macro already open")
			mtabs.Select(d)
			return
		}
	}
	t := container.NewTabItem(m.Name, NewMacroTree(m))
	mtabs.AddTab(m.Name, t)
	setMacroTree(mtabs.SelectedTab())
	syncMacroToolbarFieldsFromSelection()
	services.RegisterMacroHotkey(m)
}

func syncMacroToolbarFieldsFromSelection() {
	mtabs := activeWire.Mui.MTabs
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

func setMtabSettingsAndWidgets(d WireDeps) {
	mtabs := d.Mui.MTabs
	mtabs.CreateTab = func() *container.TabItem {
		name := "new macro " + uuid.NewString()
		m := models.NewMacro(name, 0, []string{})
		appdata.Macros().Set(m.Name, m)
		ti := container.NewTabItem(
			name,
			NewMacroTree(m),
		)

		setMacroTree(ti.Content.(*MacroTree))
		go fyne.DoAndWait(func() {
			mtabs.BoundMacroListWidget.Refresh()
		})
		return ti
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		m, err := appdata.Macros().Get(ti.Text)
		if err == nil {
			services.UnregisterMacroHotkey(m)
		}
		mtabs.SelectIndex(0)
		fyneui.RunOnMain(func() {
			mtabs.BoundMacroListWidget.Refresh()
		})
	}

	mtabs.OnUnselected = func(_ *container.TabItem) {
		mt := mtabs.SelectedTab()
		if mt != nil {
			mt.UnselectAll()
			mt.SelectedNode = ""
			if d.RefreshItemsAccordion != nil {
				d.RefreshItemsAccordion()
			}
		}
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		if _, err := appdata.Macros().Get(ti.Text); err != nil {
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
		if err := appdata.Macros().Set(mt.Macro.Name, mt.Macro); err != nil {
			log.Printf("save hotkey: persist macro: %v", err)
		}
	}
	mtabs.MacroHotkeyRecordBtn.OnTapped = func() {
		d.ShowHotkeyRecordDialog(d.Window, 1*time.Second, func(keys []string) {
			mtabs.MacroHotkeyLabel.SetText(services.ReverseParseMacroHotkey(keys))
			saveHotkey(true)
		})
	}
	mtabs.HotkeyTriggerRadio.OnChanged = func(string) {
		saveHotkey(false)
	}

	mtabs.MacroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			e := dialog.NewError(errors.New("macro name cannot be empty"), d.Window)
			d.AddDialogEscapeClose(e, d.Window)
			e.Show()
			return
		}
		for _, m := range appdata.Macros().GetAll() {
			if m.Name == sub {
				d.ShowErrorWithEscape(errors.New("macro name already exists"), d.Window)
				return
			}
		}

		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}

		appdata.Macros().Delete(mt.Macro.Name)

		mt.Macro.Name = sub
		mtabs.Selected().Text = sub

		appdata.Macros().Set(mt.Macro.Name, mt.Macro)

		mtabs.BoundMacroListWidget.Refresh()
		mtabs.Refresh()
	}
	mtabs.BoundGlobalDelayEntry.OnChanged = func(gd int) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		mt.Macro.GlobalDelay = gd
		applyMacroGlobalDelays(gd)
		appdata.Macros().Set(mt.Macro.Name, mt.Macro)
	}
}

func setMacroSelect(b *widget.Button, d WireDeps) {
	var popup *widget.PopUp
	b.Text = ""
	b.Icon = theme.ListIcon()
	b.OnTapped = func() {
		d.Mui.MTabs.BoundMacroListWidget = widget.NewList(
			func() int {
				return len(appdata.Macros().GetAll())
			},
			func() fyne.CanvasObject {
				return container.NewHBox(widget.NewLabel("Template"), layout.NewSpacer(), &widget.Button{Icon: theme.CancelIcon(), Importance: widget.LowImportance})
			},
			func(id widget.ListItemID, co fyne.CanvasObject) {
				fyneui.RunOnMain(func() {
					k := appdata.Macros().GetAllKeys()
					c := co.(*fyne.Container)
					label := c.Objects[0].(*widget.Label)
					removeButton := c.Objects[2].(*widget.Button)
					slices.Sort(k)
					v := k[id]
					label.SetText(v)
					label.Importance = widget.MediumImportance
					for _, tab := range d.Mui.MTabs.Items {
						if tab.Text == v {
							label.Importance = widget.SuccessImportance
						}
					}
					label.Refresh()
					removeButton.OnTapped = func() {
						m, err := appdata.Macros().Get(v)
						if err != nil {
							log.Printf("Error getting macro %s: %v", v, err)
							return
						}
						d.ShowConfirmWithEscape("Delete Macro", "Are you sure you want to delete this macro?", func(ok bool) {
							if ok {
								if err := appdata.Macros().Delete(v); err != nil {
									log.Printf("Error deleting macro %s: %v", v, err)
									return
								}
								d.Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
								for _, ti := range d.Mui.MTabs.Items {
									if m.Name == ti.Text {
										d.Mui.MTabs.Remove(ti)
									}
								}
								d.Mui.MTabs.Refresh()
							}
						}, d.Window)
					}
					removeButton.Show()
				})
			},
		)
		d.Mui.MTabs.BoundMacroListWidget.OnSelected =
			func(id widget.ListItemID) {
				k := appdata.Macros().GetAllKeys()
				slices.Sort(k)
				log.Println(k[id])
				m, err := appdata.Macros().Get(k[id])
				if err != nil {
					log.Printf("Error getting macro %s: %v", k[id], err)
					return
				}
				AddMacroTab(m)
				d.Mui.MTabs.BoundMacroListWidget.RefreshItem(id)
				d.Mui.MTabs.BoundMacroListWidget.UnselectAll()
			}
		popUpContent := container.NewBorder(
			widget.NewButton("Close", func() {
				popup.Hide()
			}), nil, nil, nil,
			container.NewAdaptiveGrid(1,
				d.Mui.MTabs.BoundMacroListWidget,
			),
		)
		popup = widget.NewModalPopUp(popUpContent, d.Window.Canvas())
		popup.Resize(fyne.NewSize(300, 500))
		popup.Show()
	}
}

func setMacroTree(mt *MacroTree) {
	if mt == nil {
		return
	}
	d := activeWire
	mt.Tree.OnSelected = func(uid widget.TreeNodeID) {
		if st := d.Mui.MTabs.SelectedTab(); st != nil {
			st.SelectedNode = uid
		}
	}
	mt.OnOpenActionDialog = func(action actions.ActionInterface) {
		if action == nil {
			return
		}
		uid := action.GetUID()
		d.ShowActionDialog(action, func(updatedAction actions.ActionInterface) {
			if err := appdata.Macros().Set(mt.Macro.Name, mt.Macro); err != nil {
				log.Printf("failed to save macro after action edit: %v", err)
			}
			mt.RefreshItem(uid)
			mt.Refresh()
		})
	}
}
