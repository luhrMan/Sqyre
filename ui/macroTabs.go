package ui

import (
	"log"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

type MacroTabs struct {
	container.DocTabs

	BoundMacroListWidget  *widget.List
	MacroNameEntry        *widget.Entry
	BoundGlobalDelayEntry *widget.Entry
	MacroHotkeyEntry      *widget.Entry
}

func NewMacroTabs() *MacroTabs {
	t := &MacroTabs{
		BoundMacroListWidget:  &widget.List{},
		MacroNameEntry:        widget.NewEntry(),
		BoundGlobalDelayEntry: widget.NewEntry(),
		MacroHotkeyEntry:      widget.NewEntry(),
	}
	t.ExtendBaseWidget(t)

	return t
}

func (mtabs *MacroTabs) AddTab(name string, t *container.TabItem) {
	//check if already open. if it is, select it.
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
	return mtabs.Selected().Content.(*MacroTree)
}
