package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VarRefEntry is a VarEntry with a visible insert-variable button for ${name} references.
type VarRefEntry struct {
	widget.BaseWidget
	VarEntry
	insert *ttwidget.Button
}

func newVarRefEntryBase(getVars func() []string, multiLine bool) *VarRefEntry {
	e := &VarRefEntry{VarEntry: VarEntry{GetVariables: getVars}}
	if multiLine {
		e.MultiLine = true
		e.Wrapping = fyne.TextWrapWord
	}
	e.VarEntry.ExtendBaseWidget(&e.VarEntry)
	e.insert = ttwidget.NewButtonWithIcon("", theme.ContentAddIcon(), func() {
		e.showVariableMenu()
	})
	e.insert.Importance = widget.LowImportance
	e.insert.SetToolTip("Insert variable reference (${name})")
	e.updateInsertButton()
	e.ExtendBaseWidget(e)
	return e
}

// NewVarRefEntry creates a single-line entry for values that may contain ${variable} refs.
func NewVarRefEntry(getVars func() []string) *VarRefEntry {
	return newVarRefEntryBase(getVars, false)
}

// NewMultiLineVarRefEntry creates a multi-line VarRefEntry.
func NewMultiLineVarRefEntry(getVars func() []string) *VarRefEntry {
	return newVarRefEntryBase(getVars, true)
}

func (e *VarRefEntry) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(container.NewBorder(nil, nil, nil, e.insert, &e.VarEntry))
}

func (e *VarRefEntry) updateInsertButton() {
	if e.insert == nil {
		return
	}
	if e.GetVariables != nil && len(e.GetVariables()) > 0 {
		e.insert.Enable()
		return
	}
	e.insert.Disable()
}

func (e *VarRefEntry) showVariableMenu() {
	e.updateInsertButton()
	if e.GetVariables == nil {
		return
	}
	vars := e.GetVariables()
	if len(vars) == 0 {
		return
	}
	items := make([]*fyne.MenuItem, len(vars))
	for i, v := range vars {
		name := v
		items[i] = fyne.NewMenuItem(name, func() {
			e.insertVariable(name)
		})
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(e)
	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(e)
	widget.ShowPopUpMenuAtPosition(fyne.NewMenu("", items...), c, pos.Add(fyne.NewPos(0, e.Size().Height)))
}
