package macro

import (
	"Sqyre/internal/config"
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"encoding/json"
	"errors"
	"log"
	"slices"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/google/uuid"
)

// WireDeps supplies window, macro UI shell, and dialog callbacks from package ui (avoids import cycle).
type WireDeps struct {
	Window                 fyne.Window
	Mui                    *MacroUi
	RefreshItemsAccordion  func()
	ShowHotkeyRecordDialog func(parent fyne.Window, stableDuration time.Duration, onRecorded func(keys []string))
	ShowErrorWithEscape    func(err error, parent fyne.Window)
	AddDialogEscapeClose   func(d dialog.Dialog, parent fyne.Window)
	AddPopupEscapeClose    func(pop *widget.PopUp, parent fyne.Window) dialog.Dialog
	ShowConfirmWithEscape  func(title, message string, callback func(bool), parent fyne.Window)
	ShowActionDialog       func(action actions.ActionInterface, onSave func(actions.ActionInterface), onCancel func())
	ShowAddActionPicker    func()
	WrapTagChip            func(inner fyne.CanvasObject) fyne.CanvasObject
}

var activeWire WireDeps

// SetMacroUi wires macro tabs, toolbar handlers, and restores open macros from preferences.
func SetMacroUi(d WireDeps) {
	activeWire = d
	setMtabSettingsAndWidgets(d)

	eager := config.IsUITestMode()
	if names := getOpenMacroNames(); len(names) > 0 {
		for _, name := range names {
			m, err := repositories.MacroRepo().Get(name)
			if err != nil {
				log.Printf("Could not reopen macro %s: %v", name, err)
				continue
			}
			addMacroTab(m, eager)
		}
	} else {
		for _, m := range repositories.MacroRepo().GetAll() {
			addMacroTab(m, eager)
			break
		}
	}
	if sel := d.Mui.MTabs.Selected(); sel != nil {
		ensureMacroTabContent(sel.Content)
	}
	setMacroSelect(d.Mui.MacroSelectButton, d)
	wireMacroTagHandlers(d.Mui.MTabs)
	wireMacroDelayHandlers(d.Mui.MTabs)
	syncMacroToolbarFieldsFromSelection()
	registerMacroTreeShortcuts(d)
}

// SaveOpenMacros persists which macro tabs are open.
func SaveOpenMacros() {
	if activeWire.Mui == nil || activeWire.Mui.MTabs == nil {
		return
	}
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
	addMacroTab(m, true)
}

// OpenMacroTabs opens each named macro in a tab. Already-open tabs are unchanged.
// Tab content is built lazily until selected.
func OpenMacroTabs(names []string) {
	mtabs := activeWire.Mui.MTabs
	open := make(map[string]bool, len(mtabs.Items))
	for _, ti := range mtabs.Items {
		open[ti.Text] = true
	}
	for _, name := range names {
		if open[name] {
			continue
		}
		m, err := repositories.MacroRepo().Get(name)
		if err != nil {
			log.Printf("Error getting macro %s: %v", name, err)
			continue
		}
		addMacroTab(m, false)
		open[name] = true
	}
}

// CloseMacroTabs closes tabs for each named macro that is currently open.
func CloseMacroTabs(names []string) {
	if len(names) == 0 {
		return
	}
	mtabs := activeWire.Mui.MTabs
	toClose := make(map[string]bool, len(names))
	for _, name := range names {
		toClose[name] = true
	}
	var items []*container.TabItem
	for _, ti := range mtabs.Items {
		if toClose[ti.Text] {
			items = append(items, ti)
		}
	}
	for _, ti := range items {
		mtabs.Remove(ti)
	}
}

func addMacroTab(m *models.Macro, eagerBuild bool) {
	mtabs := activeWire.Mui.MTabs
	for _, d := range mtabs.Items {
		if d.Text == m.Name {
			log.Println("macro already open")
			mtabs.Select(d)
			if eagerBuild {
				ensureMacroTabContent(d.Content)
			}
			return
		}
	}
	var content fyne.CanvasObject
	if eagerBuild {
		built := NewMacroTabContent(m)
		content = built
		t := container.NewTabItem(m.Name, content)
		mtabs.AddTab(m.Name, t)
		setMacroTree(built.Tree)
	} else {
		content = NewLazyMacroTabHost(m)
		t := container.NewTabItem(m.Name, content)
		mtabs.AddTab(m.Name, t)
	}
	syncMacroToolbarFieldsFromSelection()
	macrohotkey.RegisterMacroHotkey(m)
}

// openMacroTabForLog opens a macro tab like AddMacroTab but does NOT register the
// macro hotkey. Used when a running macro (possibly hotkey-triggered) needs its
// tab opened to show the execution log; re-registering here would duplicate the
// hotkey that just fired.
func openMacroTabForLog(m *models.Macro) {
	mtabs := activeWire.Mui.MTabs
	for _, d := range mtabs.Items {
		if d.Text == m.Name {
			mtabs.Select(d)
			ensureMacroTabContent(d.Content)
			syncMacroToolbarFieldsFromSelection()
			return
		}
	}
	built := NewMacroTabContent(m)
	t := container.NewTabItem(m.Name, built)
	mtabs.AddTab(m.Name, t)
	setMacroTree(built.Tree)
	syncMacroToolbarFieldsFromSelection()
}

func syncMacroToolbarFieldsFromSelection() {
	mtabs := activeWire.Mui.MTabs
	st := mtabs.SelectedTab()
	if mtabs.macroDelayPopup != nil {
		mtabs.macroDelayPopup.Hide()
		mtabs.macroDelayPopup = nil
	}
	if st == nil || st.Macro == nil {
		return
	}
	mtabs.MacroNameEntry.SetText(st.Macro.Name)
	mtabs.BoundGlobalDelayEntry.SetValue(st.Macro.GlobalDelay)
	mtabs.BoundKeyboardDelayEntry.SetValue(st.Macro.KeyboardDelay)
	mtabs.BoundMouseDelayEntry.SetValue(st.Macro.MouseDelay)
	updateMacroDelayButton(mtabs, st.Macro)
	if len(st.Macro.Hotkey) == 0 {
		mtabs.MacroHotkeyLabel.SetText("—")
	} else {
		mtabs.MacroHotkeyLabel.SetText(macrohotkey.ReverseParseMacroHotkey(st.Macro.Hotkey))
	}
	mtabs.MacroHotkeyClearBtn.Disable()
	if len(st.Macro.Hotkey) > 0 {
		mtabs.MacroHotkeyClearBtn.Enable()
	}
	mtabs.HotkeyTriggerRadio.SetSelected(models.ParseHotkeyTrigger(st.Macro.HotkeyTrigger).UILabel())
	updateMacroTagsDisplay(mtabs, st.Macro)
	if mtabs.MacroTagEntry != nil {
		mtabs.MacroTagEntry.SetText("")
		mtabs.MacroTagEntry.HideCompletion()
	}
}

func setMtabSettingsAndWidgets(d WireDeps) {
	mtabs := d.Mui.MTabs
	mtabs.CreateTab = func() *container.TabItem {
		name := "new macro " + uuid.NewString()
		m := models.NewMacro(name, 0, []string{})
		repositories.MacroRepo().Set(m.Name, m)
		ti := container.NewTabItem(
			name,
			NewMacroTabContent(m),
		)

		setMacroTree(ti.Content.(*MacroTabContent).Tree)
		go fyne.DoAndWait(func() {
			custom_widgets.RefreshListPreservingScroll(mtabs.BoundMacroListWidget)
		})
		return ti
	}
	refreshMacroTabSelection := func(ti *container.TabItem) {
		if ti == nil {
			return
		}
		if _, err := repositories.MacroRepo().Get(ti.Text); err != nil {
			log.Printf("Error getting macro %s: %v", ti.Text, err)
			return
		}
		ensureMacroTabContent(ti.Content)
		syncMacroToolbarFieldsFromSelection()
		if c := macroTabContentFrom(ti.Content); c != nil {
			c.RefreshVariablesPanel()
		}
		if mtabs.OnHistoryButtonsSync != nil {
			mtabs.OnHistoryButtonsSync()
		}
		if mtabs.OnTabMoveButtonsSync != nil {
			mtabs.OnTabMoveButtonsSync()
		}
	}

	mtabs.OnClosed = func(ti *container.TabItem) {
		if m := macroFromTabContent(ti.Content); m != nil {
			macrohotkey.UnregisterMacroHotkey(m)
		}
		mtabs.SelectIndex(0)
		// Fyne does not fire OnSelected when the remaining tab is already at index 0
		// (common when closing the first/selected tab), so refresh toolbar fields here.
		refreshMacroTabSelection(mtabs.Selected())
		custom_widgets.RefreshListPreservingScroll(mtabs.BoundMacroListWidget)
		SaveOpenMacros()
	}

	mtabs.OnUnselected = func(_ *container.TabItem) {
		mt := mtabs.SelectedTab()
		if mt != nil {
			unselectMacroTreeAction(mt)
			if d.RefreshItemsAccordion != nil {
				d.RefreshItemsAccordion()
			}
		}
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		refreshMacroTabSelection(ti)
	}

	saveHotkey := func(deferHookRegister bool) {
		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}
		oldHk := slices.Clone(mt.Macro.Hotkey)
		oldTr := mt.Macro.HotkeyTrigger
		if !deferHookRegister {
			macrohotkey.UnregisterHotkeyKeys(oldHk, oldTr)
		}

		disp := mtabs.MacroHotkeyLabel.Text
		if disp == "" || disp == "—" {
			mt.Macro.Hotkey = []string{}
		} else {
			mt.Macro.Hotkey = macrohotkey.ParseMacroHotkey(disp)
		}
		mt.Macro.HotkeyTrigger = string(models.HotkeyTriggerFromUILabel(mtabs.HotkeyTriggerRadio.Selected))

		if !deferHookRegister {
			macrohotkey.RegisterMacroHotkey(mt.Macro)
		}
		if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
			log.Printf("save hotkey: persist macro: %v", err)
		}
	}
	mtabs.MacroHotkeyRecordBtn.OnTapped = func() {
		d.ShowHotkeyRecordDialog(d.Window, 1*time.Second, func(keys []string) {
			mtabs.MacroHotkeyLabel.SetText(macrohotkey.ReverseParseMacroHotkey(keys))
			saveHotkey(true)
			syncMacroToolbarFieldsFromSelection()
		})
	}
	mtabs.MacroHotkeyClearBtn.OnTapped = func() {
		mt := mtabs.SelectedTab()
		if mt == nil || len(mt.Macro.Hotkey) == 0 {
			return
		}
		mtabs.MacroHotkeyLabel.SetText("—")
		saveHotkey(false)
		syncMacroToolbarFieldsFromSelection()
	}
	mtabs.HotkeyTriggerRadio.OnChanged = func(string) {
		saveHotkey(false)
	}

	mtabs.MacroNameEntry.OnSubmitted = func(sub string) {
		if sub == "" {
			d.ShowErrorWithEscape(errors.New("macro name cannot be empty"), d.Window)
			return
		}
		for _, m := range repositories.MacroRepo().GetAll() {
			if m.Name == sub {
				d.ShowErrorWithEscape(errors.New("macro name already exists"), d.Window)
				return
			}
		}

		mt := mtabs.SelectedTab()
		if mt == nil {
			return
		}

		oldName := mt.Macro.Name
		repositories.MacroRepo().Delete(oldName)

		mt.Macro.Name = sub
		mtabs.Selected().Text = sub

		repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro)
		if _, err := repositories.PropagateMacroRename(oldName, sub); err != nil {
			log.Printf("propagate macro rename %q -> %q: %v", oldName, sub, err)
		}
		for _, tree := range mtabs.AllTrees() {
			tree.Refresh()
		}

		custom_widgets.RefreshListPreservingScroll(mtabs.BoundMacroListWidget)
		mtabs.Refresh()
	}
}

func setMacroSelect(b *widget.Button, d WireDeps) {
	b.Text = ""
	b.Icon = theme.ListIcon()
	b.OnTapped = func() {
		showMacroListPopup(d)
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
	mt.Tree.OnUnselected = func(uid widget.TreeNodeID) {
		if st := d.Mui.MTabs.SelectedTab(); st != nil && st.SelectedNode == string(uid) {
			st.SelectedNode = ""
		}
	}
	mt.OnTreeChanged = func() {
		if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
			log.Printf("failed to save macro after tree change: %v", err)
		}
		if c := d.Mui.MTabs.SelectedMacroContent(); c != nil {
			c.RefreshVariablesPanel()
		}
	}
	mt.OnHistoryChanged = func() {
		if d.Mui.MTabs.OnHistoryButtonsSync != nil {
			d.Mui.MTabs.OnHistoryButtonsSync()
		}
	}
	mt.OnOpenActionDialog = func(action actions.ActionInterface) {
		if action == nil {
			return
		}
		uid := action.GetUID()
		mt.RecordMutation()
		d.ShowActionDialog(action, func(updatedAction actions.ActionInterface) {
			if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
				log.Printf("failed to save macro after action edit: %v", err)
			}
			mt.RefreshItem(uid)
			mt.Refresh()
			if c := d.Mui.MTabs.SelectedMacroContent(); c != nil {
				c.RefreshVariablesPanel()
			}
		}, nil)
	}
	mt.onShowAddActionPicker = func() {
		if d.ShowAddActionPicker != nil {
			d.ShowAddActionPicker()
		}
	}
}

func handleMacroTreeShortcut(mt *MacroTree, shortcut fyne.Shortcut) {
	if mt == nil {
		return
	}
	switch shortcut.(type) {
	case *fyne.ShortcutSelectAll:
		if mt.onShowAddActionPicker != nil {
			mt.onShowAddActionPicker()
		}
	case *fyne.ShortcutCopy:
		copyMacroTreeSelection(mt)
	case *fyne.ShortcutPaste:
		pasteMacroTreeClipboard(mt)
	case *fyne.ShortcutUndo:
		mt.Undo()
	case *fyne.ShortcutRedo:
		mt.Redo()
	default:
		custom, ok := shortcut.(*desktop.CustomShortcut)
		if !ok {
			return
		}
		switch custom.KeyName {
		case fyne.KeyD:
			if custom.Modifier == fyne.KeyModifierControl {
				mt.DeleteSelectedAction()
			}
		case fyne.KeyUp:
			if custom.Modifier == fyne.KeyModifierAlt {
				moveMacroTreeSelection(mt, true)
			}
		case fyne.KeyDown:
			if custom.Modifier == fyne.KeyModifierAlt {
				moveMacroTreeSelection(mt, false)
			}
		}
	}
}

func registerMacroTreeShortcuts(d WireDeps) {
	if d.Window == nil {
		return
	}
	canvas := d.Window.Canvas()
	canvas.AddShortcut(&fyne.ShortcutUndo{}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&fyne.ShortcutRedo{}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&fyne.ShortcutCopy{}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&fyne.ShortcutPaste{}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&desktop.CustomShortcut{
		KeyName:  fyne.KeyUp,
		Modifier: fyne.KeyModifierAlt,
	}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&desktop.CustomShortcut{
		KeyName:  fyne.KeyDown,
		Modifier: fyne.KeyModifierAlt,
	}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&desktop.CustomShortcut{
		KeyName:  fyne.KeyD,
		Modifier: fyne.KeyModifierControl,
	}, func(shortcut fyne.Shortcut) {
		handleMacroTreeShortcut(d.Mui.MTabs.SelectedTab(), shortcut)
	})
	canvas.AddShortcut(&desktop.CustomShortcut{
		KeyName: fyne.KeyEscape,
	}, func(fyne.Shortcut) {
		if services.ShouldEscapeStopMacro() {
			services.RequestMacroStop()
		}
	})
}
