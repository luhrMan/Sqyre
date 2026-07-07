package macro

import (
	"log"
	"slices"

	"Sqyre/ui/completionentry"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

type MacroTabs struct {
	container.DocTabs

	BoundMacroListWidget  *widget.List
	MacroNameEntry        *widget.Entry
	BoundGlobalDelayEntry    *custom_widgets.Incrementer
	BoundKeyboardDelayEntry  *custom_widgets.Incrementer
	BoundMouseDelayEntry     *custom_widgets.Incrementer
	delayMin                 int
	delayMax                 int
	MacroDelayBtn            *ttwidget.Button
	macroDelayPopup          *widget.PopUp
	MacroHotkeyLabel      *widget.Label
	MacroHotkeyRecordBtn  *widget.Button
	MacroHotkeyClearBtn   *widget.Button
	HotkeyTriggerRadio    *widget.RadioGroup
	MacroTagEntry         *completionentry.CompletionEntry
	MacroTagSubmitBtn     *widget.Button
	MacroTagsBtn          *ttwidget.Button
	MacroTagsContainer    *fyne.Container
	macroTagsPopup        *widget.PopUp

	// OnHistoryButtonsSync updates undo/redo toolbar button enabled state.
	OnHistoryButtonsSync func()
	// OnTabMoveButtonsSync updates tab move button enabled state.
	OnTabMoveButtonsSync func()
}

func NewMacroTabs() *MacroTabs {
	hkLabel := widget.NewLabel("—")
	hkLabel.Wrapping = fyne.TextWrapOff
	hkLabel.TextStyle = fyne.TextStyle{Monospace: true}

	tagEntry := completionentry.NewCompletionEntry([]string{})
	tagEntry.PlaceHolder = "Add tag…"
	tagSubmitBtn := widget.NewButtonWithIcon("", theme.ContentAddIcon(), nil)
	tagSubmitBtn.Importance = widget.MediumImportance
	tagsBtn := ttwidget.NewButtonWithIcon("", theme.InfoIcon(), nil)

	delayBtn := ttwidget.NewButtonWithIcon("", theme.HistoryIcon(), nil)

	t := &MacroTabs{
		BoundMacroListWidget: &widget.List{},
		MacroNameEntry:       custom_widgets.NewFormEntry(),
		delayMin:             0,
		delayMax:             1000,
		MacroDelayBtn:        delayBtn,
		MacroHotkeyLabel:     hkLabel,
		MacroHotkeyRecordBtn: widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil),
		MacroHotkeyClearBtn:  widget.NewButtonWithIcon("", theme.ContentClearIcon(), nil),
		HotkeyTriggerRadio:   widget.NewRadioGroup([]string{"On press", "On release"}, nil),
		MacroTagEntry:        tagEntry,
		MacroTagSubmitBtn:    tagSubmitBtn,
		MacroTagsBtn:         tagsBtn,
		MacroTagsContainer:   newMacroTagsContainer(),
	}
	t.MacroHotkeyClearBtn.Importance = widget.LowImportance
	t.HotkeyTriggerRadio.Horizontal = true
	t.HotkeyTriggerRadio.Required = true
	t.HotkeyTriggerRadio.SetSelected("On press")
	t.BoundGlobalDelayEntry = custom_widgets.NewIncrementerWithEntry(0, 1, &t.delayMin, &t.delayMax)
	t.BoundKeyboardDelayEntry = custom_widgets.NewIncrementerWithEntry(0, 1, &t.delayMin, &t.delayMax)
	t.BoundMouseDelayEntry = custom_widgets.NewIncrementerWithEntry(0, 1, &t.delayMin, &t.delayMax)
	t.ExtendBaseWidget(t)

	return t
}

func (mtabs *MacroTabs) AddTab(name string, t *container.TabItem) {
	for _, d := range mtabs.Items {
		if d.Text == name {
			log.Println("macro already open")
			mtabs.Select(d)
			return
		}
	}
	mtabs.Append(t)
	mtabs.Select(t)
}

// SelectedTab returns the currently selected macro tree, or nil if no tab is open.
func (mtabs *MacroTabs) SelectedTab() *MacroTree {
	if mtabs.Selected() == nil {
		return nil
	}
	if c := ensureMacroTabContent(mtabs.Selected().Content); c != nil {
		return c.Tree
	}
	if tree, ok := mtabs.Selected().Content.(*MacroTree); ok {
		return tree
	}
	return nil
}

// TreeForMacro returns the open tab's tree for the given macro name, or nil.
func (mtabs *MacroTabs) TreeForMacro(name string) *MacroTree {
	for _, item := range mtabs.Items {
		if item.Text != name {
			continue
		}
		if c := ensureMacroTabContent(item.Content); c != nil {
			return c.Tree
		}
		if tree, ok := item.Content.(*MacroTree); ok {
			return tree
		}
	}
	return nil
}

// AllTrees returns every open macro tree (built tabs only).
func (mtabs *MacroTabs) AllTrees() []*MacroTree {
	trees := make([]*MacroTree, 0, len(mtabs.Items))
	for _, item := range mtabs.Items {
		if c := macroTabContentFrom(item.Content); c != nil && c.Tree != nil {
			trees = append(trees, c.Tree)
			continue
		}
		if tree, ok := item.Content.(*MacroTree); ok {
			trees = append(trees, tree)
		}
	}
	return trees
}

// RefreshActionDisplayColors rebuilds open macro trees and variable rows so
// action icon backgrounds and display pills pick up new colors.
func (mtabs *MacroTabs) RefreshActionDisplayColors() {
	for _, item := range mtabs.Items {
		if c := macroTabContentFrom(item.Content); c != nil {
			if c.Tree != nil {
				c.Tree.Refresh()
			}
			if c.VariablesPanel != nil {
				c.VariablesPanel.RefreshDefs()
			}
			continue
		}
		if tree, ok := item.Content.(*MacroTree); ok && tree != nil {
			tree.Refresh()
		}
	}
}

// SelectedMacroContent returns the full tab content wrapper when present.
func (mtabs *MacroTabs) SelectedMacroContent() *MacroTabContent {
	if mtabs.Selected() == nil {
		return nil
	}
	return ensureMacroTabContent(mtabs.Selected().Content)
}

// CanMoveSelectedTab reports whether the selected tab can move by delta slots.
func (mtabs *MacroTabs) CanMoveSelectedTab(delta int) bool {
	if delta == 0 || len(mtabs.Items) < 2 {
		return false
	}
	from := mtabs.SelectedIndex()
	if from < 0 || from >= len(mtabs.Items) {
		return false
	}
	to := from + delta
	return to >= 0 && to < len(mtabs.Items)
}

// MoveSelectedTab reorders the selected tab by delta slots.
func (mtabs *MacroTabs) MoveSelectedTab(delta int) bool {
	if !mtabs.CanMoveSelectedTab(delta) {
		return false
	}
	from := mtabs.SelectedIndex()
	to := from + delta
	selected := mtabs.Selected()
	if selected == nil {
		return false
	}
	items := slices.Clone(mtabs.Items)
	item := items[from]
	items = append(items[:from], items[from+1:]...)
	if to >= len(items) {
		items = append(items, item)
	} else {
		items = append(items[:to], append([]*container.TabItem{item}, items[to:]...)...)
	}
	mtabs.SetItems(items)
	mtabs.Select(selected)
	if mtabs.OnTabMoveButtonsSync != nil {
		mtabs.OnTabMoveButtonsSync()
	}
	return true
}
