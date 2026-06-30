package macro

import (
	"log"

	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type MacroTabs struct {
	container.DocTabs

	BoundMacroListWidget  *widget.List
	MacroNameEntry        *widget.Entry
	BoundGlobalDelayEntry *custom_widgets.Incrementer
	globalDelayMin        int
	globalDelayMax        int
	MacroHotkeyLabel      *widget.Label
	MacroHotkeyRecordBtn  *widget.Button
	HotkeyTriggerRadio    *widget.RadioGroup
}

func NewMacroTabs() *MacroTabs {
	hkLabel := widget.NewLabel("—")
	hkLabel.Wrapping = fyne.TextWrapOff
	hkLabel.TextStyle = fyne.TextStyle{Monospace: true}

	t := &MacroTabs{
		BoundMacroListWidget: &widget.List{},
		MacroNameEntry:       widget.NewEntry(),
		globalDelayMin:       0,
		globalDelayMax:       1000,
		MacroHotkeyLabel:     hkLabel,
		MacroHotkeyRecordBtn: widget.NewButtonWithIcon("", theme.MediaRecordIcon(), nil),
		HotkeyTriggerRadio:   widget.NewRadioGroup([]string{"On press", "On release"}, nil),
	}
	t.HotkeyTriggerRadio.Horizontal = true
	t.HotkeyTriggerRadio.Required = true
	t.HotkeyTriggerRadio.SetSelected("On press")
	t.BoundGlobalDelayEntry = custom_widgets.NewIncrementerWithEntry(0, 1, &t.globalDelayMin, &t.globalDelayMax)
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

// SelectedMacroContent returns the full tab content wrapper when present.
func (mtabs *MacroTabs) SelectedMacroContent() *MacroTabContent {
	if mtabs.Selected() == nil {
		return nil
	}
	return ensureMacroTabContent(mtabs.Selected().Content)
}
