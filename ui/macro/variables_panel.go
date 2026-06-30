package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VariablesPanel lists macro variables, their sources, and optional initial values.
type VariablesPanel struct {
	list     *widget.List
	macro    *models.Macro
	defs     []models.VariableDef
	onChange func()
}

type variableListRow struct {
	nameLbl      *widget.Label
	sourceLbl    *widget.Label
	initialEntry *widget.Entry
	editBtn      *ttwidget.Button
	removeBtn    *ttwidget.Button
}

func newVariableListRow() (fyne.CanvasObject, *variableListRow) {
	row := &variableListRow{
		nameLbl:      widget.NewLabel(""),
		sourceLbl:    widget.NewLabel(""),
		initialEntry: widget.NewEntry(),
		editBtn:      ttwidget.NewButtonWithIcon("", theme.DocumentCreateIcon(), nil),
		removeBtn:    ttwidget.NewButtonWithIcon("", theme.DeleteIcon(), nil),
	}
	row.sourceLbl.Wrapping = fyne.TextTruncate
	row.initialEntry.SetPlaceHolder("initial value (optional)")
	row.editBtn.Importance = widget.LowImportance
	row.removeBtn.Importance = widget.DangerImportance
	root := container.NewBorder(nil, nil, nil,
		container.NewHBox(row.editBtn, row.removeBtn),
		container.NewVBox(row.nameLbl, row.sourceLbl, row.initialEntry),
	)
	return root, row
}

func newVariablesPanel(m *models.Macro, onChange func()) *VariablesPanel {
	p := &VariablesPanel{macro: m, onChange: onChange}
	p.list = widget.NewList(
		func() int { return len(p.defs) },
		func() fyne.CanvasObject {
			root, _ := newVariableListRow()
			return root
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id < 0 || id >= len(p.defs) {
				return
			}
			row, ok := variableListRowFrom(obj)
			if !ok {
				return
			}
			p.updateListRow(id, row)
		},
	)
	p.RefreshDefs()
	return p
}

func variableListRowFrom(obj fyne.CanvasObject) (*variableListRow, bool) {
	border, ok := obj.(*fyne.Container)
	if !ok {
		return nil, false
	}
	var vbox, hbox *fyne.Container
	for _, child := range border.Objects {
		c, ok := child.(*fyne.Container)
		if !ok || len(c.Objects) == 0 {
			continue
		}
		switch c.Objects[0].(type) {
		case *widget.Label:
			vbox = c
		case *ttwidget.Button:
			hbox = c
		}
	}
	if vbox == nil || len(vbox.Objects) < 3 {
		return nil, false
	}
	nameLbl, ok := vbox.Objects[0].(*widget.Label)
	if !ok {
		return nil, false
	}
	sourceLbl, ok := vbox.Objects[1].(*widget.Label)
	if !ok {
		return nil, false
	}
	initialEntry, ok := vbox.Objects[2].(*widget.Entry)
	if !ok {
		return nil, false
	}
	var editBtn, removeBtn *ttwidget.Button
	if hbox != nil {
		if len(hbox.Objects) > 0 {
			editBtn, _ = hbox.Objects[0].(*ttwidget.Button)
		}
		if len(hbox.Objects) > 1 {
			removeBtn, _ = hbox.Objects[1].(*ttwidget.Button)
		}
	}
	return &variableListRow{
		nameLbl:      nameLbl,
		sourceLbl:    sourceLbl,
		initialEntry: initialEntry,
		editBtn:      editBtn,
		removeBtn:    removeBtn,
	}, true
}

func variableIsRemovable(d models.VariableDef) bool {
	return d.Role != models.VariableRoleBuiltin && d.Source.ActionType == "initial"
}

func (p *VariablesPanel) updateListRow(id widget.ListItemID, row *variableListRow) {
	d := p.defs[id]
	row.nameLbl.SetText(d.Name)
	role := string(d.Role)
	if d.Source.Conditional {
		role += " (conditional)"
	}
	row.sourceLbl.SetText(fmt.Sprintf("%s · %s · %s", d.Source.ActionType, d.Source.ActionName, role))
	row.initialEntry.SetText(d.InitialValue)
	row.initialEntry.OnChanged = func(text string) {
		if p.macro == nil {
			return
		}
		p.macro.SetInitialVariable(d.Name, strings.TrimSpace(text))
		_ = repositories.MacroRepo().Set(p.macro.Name, p.macro)
		if p.onChange != nil {
			p.onChange()
		}
	}
	if row.editBtn != nil {
		row.editBtn.SetToolTip("Open source action")
		uid := d.Source.ActionUID
		row.editBtn.OnTapped = func() {
			if uid == "" || p.macro == nil {
				return
			}
			action := p.macro.FindActionByUID(uid)
			if action == nil {
				return
			}
			if st := activeWire.Mui.MTabs.SelectedTab(); st != nil {
				st.Select(uid)
				st.OnOpenActionDialog(action)
			}
		}
		if uid == "" {
			row.editBtn.Disable()
		} else {
			row.editBtn.Enable()
		}
	}
	if row.removeBtn != nil {
		row.removeBtn.SetToolTip("Remove variable")
		name := d.Name
		row.removeBtn.OnTapped = func() {
			if !variableIsRemovable(d) || p.macro == nil {
				return
			}
			w := activeWire.Window
			msg := fmt.Sprintf("Remove variable %q?", name)
			dlg := dialog.NewConfirm("Remove variable", msg, func(ok bool) {
				if !ok {
					return
				}
				p.macro.SetInitialVariable(name, "")
				_ = repositories.MacroRepo().Set(p.macro.Name, p.macro)
				p.RefreshDefs()
				if p.onChange != nil {
					p.onChange()
				}
			}, w)
			if activeWire.AddDialogEscapeClose != nil {
				activeWire.AddDialogEscapeClose(dlg, w)
			}
			dlg.Show()
		}
		if variableIsRemovable(d) {
			row.removeBtn.Enable()
		} else {
			row.removeBtn.Disable()
		}
	}
}

// RefreshDefs reloads variable definitions from the macro.
func (p *VariablesPanel) RefreshDefs() {
	if p.macro == nil {
		p.defs = nil
	} else {
		p.defs = p.macro.CollectVariableDefs()
	}
	if p.list != nil {
		p.list.Refresh()
	}
}

func variablesPanelChrome(panel *VariablesPanel, m *models.Macro) fyne.CanvasObject {
	addBtn := widget.NewButtonWithIcon("Add variable", theme.ContentAddIcon(), func() {
		if m == nil {
			return
		}
		nameEntry := widget.NewEntry()
		nameEntry.SetPlaceHolder("Variable name")
		valueEntry := widget.NewEntry()
		valueEntry.SetPlaceHolder("Initial value (optional)")
		form := widget.NewForm(
			widget.NewFormItem("Name", nameEntry),
			widget.NewFormItem("Initial value", valueEntry),
		)
		w := activeWire.Window
		d := dialog.NewCustomConfirm("Add variable", "Add", "Cancel", form, func(ok bool) {
			if !ok {
				return
			}
			name := strings.TrimSpace(nameEntry.Text)
			if name == "" {
				return
			}
			m.SetInitialVariable(name, strings.TrimSpace(valueEntry.Text))
			_ = repositories.MacroRepo().Set(m.Name, m)
			panel.RefreshDefs()
			if panel.onChange != nil {
				panel.onChange()
			}
		}, w)
		if activeWire.AddDialogEscapeClose != nil {
			activeWire.AddDialogEscapeClose(d, w)
		}
		d.Show()
	})
	addSetBtn := widget.NewButtonWithIcon("Add Set action", theme.DocumentCreateIcon(), func() {
		if m == nil || m.Root == nil {
			return
		}
		nameEntry := widget.NewEntry()
		nameEntry.SetPlaceHolder("Variable name")
		valueEntry := widget.NewEntry()
		valueEntry.SetPlaceHolder("Value")
		form := widget.NewForm(
			widget.NewFormItem("Name", nameEntry),
			widget.NewFormItem("Value", valueEntry),
		)
		w := activeWire.Window
		d := dialog.NewCustomConfirm("Add Set action", "Add", "Cancel", form, func(ok bool) {
			if !ok {
				return
			}
			name := strings.TrimSpace(nameEntry.Text)
			if name == "" {
				return
			}
			sv := actions.NewSetVariable(name, valueEntry.Text)
			m.Root.AddSubAction(sv)
			_ = repositories.MacroRepo().Set(m.Name, m)
			panel.RefreshDefs()
			if st := activeWire.Mui.MTabs.SelectedTab(); st != nil {
				st.Refresh()
			}
			if panel.onChange != nil {
				panel.onChange()
			}
		}, w)
		if activeWire.AddDialogEscapeClose != nil {
			activeWire.AddDialogEscapeClose(d, w)
		}
		d.Show()
	})
	help := widget.NewLabel("Variables are set by actions at runtime. Initial values load when the macro runs. Run macro uses an isolated variable store per macro.")
	help.Wrapping = fyne.TextWrapWord
	top := container.NewVBox(help, container.NewHBox(addBtn, addSetBtn))
	return container.NewBorder(top, nil, nil, nil, panel.list)
}
