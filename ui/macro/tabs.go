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
	return mtabs.Selected().Content.(*MacroTree)
}
