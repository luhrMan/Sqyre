package macro

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

const variableRowIconSize = 24

// VariablesPanel lists macro variables, their sources, types, initial values,
// and descriptions, and lets the user create, edit, rename, and remove them.
type VariablesPanel struct {
	list     *widget.List
	macro    *models.Macro
	defs     []models.VariableDef
	onChange func()
}

type variableListRow struct {
	iconBg     *canvas.Rectangle
	iconBtn    *ttwidget.Button
	pillsBox   *fyne.Container
	detailsBtn *ttwidget.Button
	editBtn    *ttwidget.Button
	renameBtn  *ttwidget.Button
	removeBtn  *ttwidget.Button
}

func variableTypeOptions() []string {
	out := make([]string, len(models.VariableTypes))
	for i, t := range models.VariableTypes {
		out[i] = string(t)
	}
	return out
}

func variablePillActionType(d models.VariableDef) string {
	if t := strings.TrimSpace(d.Source.ActionType); t != "" && t != "initial" {
		return t
	}
	return "setvariable"
}

func variableDisplayPills(d models.VariableDef) fyne.CanvasObject {
	line := container.NewHBox()
	actionType := variablePillActionType(d)
	line.Add(actions.NewDisplayPill("Name: "+d.Name, actionType))

	typ := d.Type
	if typ == "" {
		typ = models.VariableTypeAuto
	}
	line.Add(actions.NewDisplayPill("Type: "+string(typ), actionType))

	if v := strings.TrimSpace(d.InitialValue); v != "" {
		line.Add(actions.NewDisplayPill("Initial: "+v, actionType))
	}
	if desc := strings.TrimSpace(d.Description); desc != "" {
		line.Add(actions.NewDisplayPill("Description: "+desc, actionType))
	}

	role := string(d.Role)
	if d.Source.Conditional {
		role += " (conditional)"
	}
	source := fmt.Sprintf("%s · %s · %s", d.Source.ActionType, d.Source.ActionName, role)
	line.Add(actions.NewDisplayPill("Source: "+source, actionType))
	return line
}

func newVariableListRow() (fyne.CanvasObject, *variableListRow) {
	row := &variableListRow{}
	row.iconBg = canvas.NewRectangle(actions.ActionPastelColor("setvariable"))
	row.iconBg.CornerRadius = 6
	row.iconBg.StrokeColor = theme.ShadowColor()
	row.iconBg.StrokeWidth = 1
	row.iconBtn = ttwidget.NewButtonWithIcon("", assets.VariableIcon, nil)
	row.iconBtn.Importance = widget.LowImportance
	iconStack := container.NewStack(row.iconBg, row.iconBtn)

	row.pillsBox = container.NewHBox()
	pillsHolder := container.NewCenter(row.pillsBox)
	scrollContent := container.NewHBox(pillsHolder)
	contentScroll := container.NewHScroll(scrollContent)
	contentScroll.SetMinSize(fyne.NewSize(0, variableRowIconSize))

	row.detailsBtn = ttwidget.NewButtonWithIcon("", theme.SettingsIcon(), nil)
	row.editBtn = ttwidget.NewButtonWithIcon("", theme.DocumentCreateIcon(), nil)
	row.renameBtn = ttwidget.NewButtonWithIcon("", theme.ContentRedoIcon(), nil)
	row.removeBtn = ttwidget.NewButtonWithIcon("", theme.DeleteIcon(), nil)
	row.detailsBtn.Importance = widget.LowImportance
	row.editBtn.Importance = widget.LowImportance
	row.renameBtn.Importance = widget.LowImportance
	row.removeBtn.Importance = widget.DangerImportance

	leftSide := container.NewHBox(iconStack)
	rightSide := container.NewHBox(row.detailsBtn, row.editBtn, row.renameBtn, row.removeBtn)
	root := container.NewBorder(nil, nil, leftSide, rightSide, contentScroll)
	return root, row
}

func variableListRowFrom(obj fyne.CanvasObject) (*variableListRow, bool) {
	border, ok := obj.(*fyne.Container)
	if !ok || len(border.Objects) < 3 {
		return nil, false
	}
	leftSide, ok := border.Objects[1].(*fyne.Container)
	if !ok || len(leftSide.Objects) == 0 {
		return nil, false
	}
	iconStack, ok := leftSide.Objects[0].(*fyne.Container)
	if !ok || len(iconStack.Objects) < 2 {
		return nil, false
	}
	iconBg, ok := iconStack.Objects[0].(*canvas.Rectangle)
	if !ok {
		return nil, false
	}
	iconBtn, ok := iconStack.Objects[1].(*ttwidget.Button)
	if !ok {
		return nil, false
	}

	contentScroll, ok := border.Objects[0].(*container.Scroll)
	if !ok {
		return nil, false
	}
	scrollContent, ok := contentScroll.Content.(*fyne.Container)
	if !ok || len(scrollContent.Objects) == 0 {
		return nil, false
	}
	pillsHolder, ok := scrollContent.Objects[0].(*fyne.Container)
	if !ok || len(pillsHolder.Objects) == 0 {
		return nil, false
	}
	pillsBox, ok := pillsHolder.Objects[0].(*fyne.Container)
	if !ok {
		return nil, false
	}

	rightSide, ok := border.Objects[2].(*fyne.Container)
	if !ok || len(rightSide.Objects) < 4 {
		return nil, false
	}
	detailsBtn, ok := rightSide.Objects[0].(*ttwidget.Button)
	if !ok {
		return nil, false
	}
	editBtn, ok := rightSide.Objects[1].(*ttwidget.Button)
	if !ok {
		return nil, false
	}
	renameBtn, ok := rightSide.Objects[2].(*ttwidget.Button)
	if !ok {
		return nil, false
	}
	removeBtn, ok := rightSide.Objects[3].(*ttwidget.Button)
	if !ok {
		return nil, false
	}

	return &variableListRow{
		iconBg:     iconBg,
		iconBtn:    iconBtn,
		pillsBox:   pillsBox,
		detailsBtn: detailsBtn,
		editBtn:    editBtn,
		renameBtn:  renameBtn,
		removeBtn:  removeBtn,
	}, true
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

func variableIsRemovable(d models.VariableDef) bool {
	return d.Role != models.VariableRoleBuiltin && d.Source.ActionType == "initial"
}

func variableIsEditable(d models.VariableDef) bool {
	return d.Role != models.VariableRoleBuiltin
}

// saveMacro persists the current macro state to disk.
func (p *VariablesPanel) saveMacro() {
	if p.macro == nil {
		return
	}
	_ = repositories.MacroRepo().Set(p.macro.Name, p.macro)
}

// persistDecl upserts a declaration for name with the given fields and saves.
func (p *VariablesPanel) persistDecl(name string, typ models.VariableType, initial, desc string) {
	if p.macro == nil {
		return
	}
	p.macro.UpsertVariable(models.VariableDecl{
		Name:         name,
		Type:         typ,
		InitialValue: initial,
		Description:  desc,
	})
	p.saveMacro()
	if p.onChange != nil {
		p.onChange()
	}
}

func (p *VariablesPanel) refreshMacroTree() {
	if st := activeWire.Mui.MTabs.SelectedTab(); st != nil {
		st.Refresh()
	}
}

func (p *VariablesPanel) goToAction(uid string) bool {
	if uid == "" || p.macro == nil || p.macro.FindActionByUID(uid) == nil {
		return false
	}
	if c := activeWire.Mui.MTabs.SelectedMacroContent(); c != nil {
		c.GoToAction(uid)
		return true
	}
	if st := activeWire.Mui.MTabs.SelectedTab(); st != nil {
		st.GoToAction(uid)
		return true
	}
	return false
}

func (p *VariablesPanel) updateListRow(id widget.ListItemID, row *variableListRow) {
	d := p.defs[id]
	actionType := variablePillActionType(d)

	row.iconBg.FillColor = actions.ActionPastelColor(actionType)
	row.iconBg.Refresh()
	row.iconBtn.SetToolTip("View all usages in this macro")
	row.iconBtn.OnTapped = func() { p.showUsagesDialog(d) }

	row.pillsBox.Objects = []fyne.CanvasObject{variableDisplayPills(d)}
	row.pillsBox.Refresh()

	editable := variableIsEditable(d)
	uid := d.Source.ActionUID
	name := d.Name

	row.detailsBtn.SetToolTip("Edit type, initial value, and description")
	row.detailsBtn.OnTapped = func() { p.showEditDetailsDialog(d) }
	if editable {
		row.detailsBtn.Enable()
	} else {
		row.detailsBtn.Disable()
	}

	row.editBtn.SetToolTip("Open source action")
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

	row.renameBtn.SetToolTip("Rename variable")
	row.renameBtn.OnTapped = func() { p.showRenameDialog(name) }
	if editable {
		row.renameBtn.Enable()
	} else {
		row.renameBtn.Disable()
	}

	row.removeBtn.SetToolTip("Remove variable")
	row.removeBtn.OnTapped = func() { p.confirmRemove(name) }
	if variableIsRemovable(d) {
		row.removeBtn.Enable()
	} else {
		row.removeBtn.Disable()
	}
}

func (p *VariablesPanel) showUsagesDialog(d models.VariableDef) {
	if p.macro == nil {
		return
	}
	usages := p.macro.CollectVariableUsages(d.Name)
	w := activeWire.Window
	if len(usages) == 0 {
		dialog.ShowInformation("Variable usages", fmt.Sprintf("%q is not referenced in this macro.", d.Name), w)
		return
	}

	type usageRow struct {
		usage models.VariableUsage
	}
	rows := make([]usageRow, len(usages))
	var hideUsages func()
	list := widget.NewList(
		func() int { return len(rows) },
		func() fyne.CanvasObject {
			label := widget.NewLabel("")
			label.Wrapping = fyne.TextWrapWord
			goBtn := widget.NewButtonWithIcon("Go to", theme.NavigateNextIcon(), nil)
			goBtn.Importance = widget.LowImportance
			return container.NewBorder(nil, nil, nil, goBtn, label)
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id < 0 || id >= len(rows) {
				return
			}
			border, ok := obj.(*fyne.Container)
			if !ok || len(border.Objects) < 2 {
				return
			}
			label, ok := border.Objects[0].(*widget.Label)
			if !ok {
				return
			}
			goBtn, ok := border.Objects[1].(*widget.Button)
			if !ok {
				return
			}
			u := rows[id].usage
			label.SetText(variableUsageLine(u))
			if u.ActionUID == "" || p.macro == nil {
				goBtn.Disable()
				goBtn.OnTapped = nil
			} else {
				goBtn.Enable()
				uid := u.ActionUID
				goBtn.OnTapped = func() {
					if p.goToAction(uid) && hideUsages != nil {
						hideUsages()
					}
				}
			}
		},
	)
	for i, u := range usages {
		rows[i].usage = u
	}
	list.Refresh()

	closeBtn := widget.NewButton("Close", nil)
	scroll := container.NewVScroll(list)
	scroll.SetMinSize(fyne.NewSize(520, 280))
	title := fmt.Sprintf("Usages of %q", d.Name)
	content := container.NewBorder(nil, container.NewHBox(layout.NewSpacer(), closeBtn), nil, nil, scroll)
	dlg := dialog.NewCustomWithoutButtons(title, content, w)
	hideUsages = dlg.Hide
	closeBtn.OnTapped = hideUsages
	if activeWire.AddDialogEscapeClose != nil {
		activeWire.AddDialogEscapeClose(dlg, w)
	}
	dlg.Resize(fyne.NewSize(560, 360))
	dlg.Show()
}

func variableUsageLine(u models.VariableUsage) string {
	where := u.ActionName
	if where == "" {
		where = u.ActionType
	}
	if where == "" {
		where = "Macro"
	}
	detail := strings.TrimSpace(u.Detail)
	switch u.Kind {
	case models.VariableUsageInitial:
		if detail != "" {
			return detail
		}
		return "Initial value"
	case models.VariableUsageDefined:
		if detail != "" {
			return fmt.Sprintf("%s · %s", where, detail)
		}
		return fmt.Sprintf("%s · Defined", where)
	case models.VariableUsageRead:
		if detail != "" {
			return fmt.Sprintf("%s · %s", where, detail)
		}
		return fmt.Sprintf("%s · Read", where)
	default:
		if detail != "" {
			return fmt.Sprintf("%s · Referenced in %s", where, detail)
		}
		return fmt.Sprintf("%s · Referenced", where)
	}
}

func (p *VariablesPanel) showEditDetailsDialog(d models.VariableDef) {
	if p.macro == nil || !variableIsEditable(d) {
		return
	}
	typ := d.Type
	if typ == "" {
		typ = models.VariableTypeAuto
	}
	typeSelect := widget.NewSelect(variableTypeOptions(), nil)
	typeSelect.SetSelected(string(typ))
	initialEntry := widget.NewEntry()
	initialEntry.SetPlaceHolder("Initial value (optional)")
	initialEntry.SetText(d.InitialValue)
	descEntry := widget.NewEntry()
	descEntry.SetPlaceHolder("Description (optional)")
	descEntry.SetText(d.Description)
	form := widget.NewForm(
		widget.NewFormItem("Type", typeSelect),
		widget.NewFormItem("Initial value", initialEntry),
		widget.NewFormItem("Description", descEntry),
	)
	w := activeWire.Window
	name := d.Name
	dlg := dialog.NewCustomConfirm("Edit variable", "Save", "Cancel", form, func(ok bool) {
		if !ok {
			return
		}
		p.persistDecl(name,
			models.ParseVariableType(typeSelect.Selected),
			strings.TrimSpace(initialEntry.Text),
			strings.TrimSpace(descEntry.Text),
		)
		p.RefreshDefs()
	}, w)
	if activeWire.AddDialogEscapeClose != nil {
		activeWire.AddDialogEscapeClose(dlg, w)
	}
	dlg.Show()
}

func (p *VariablesPanel) showRenameDialog(oldName string) {
	if p.macro == nil {
		return
	}
	entry := widget.NewEntry()
	entry.SetText(oldName)
	form := widget.NewForm(widget.NewFormItem("New name", entry))
	w := activeWire.Window
	d := dialog.NewCustomConfirm("Rename variable", "Rename", "Cancel", form, func(ok bool) {
		if !ok {
			return
		}
		newName := strings.TrimSpace(entry.Text)
		if newName == "" || newName == oldName {
			return
		}
		if err := p.macro.RenameVariable(oldName, newName); err != nil {
			dialog.ShowError(err, w)
			return
		}
		p.saveMacro()
		p.RefreshDefs()
		p.refreshMacroTree()
		if p.onChange != nil {
			p.onChange()
		}
	}, w)
	if activeWire.AddDialogEscapeClose != nil {
		activeWire.AddDialogEscapeClose(d, w)
	}
	d.Show()
}

func (p *VariablesPanel) confirmRemove(name string) {
	if p.macro == nil {
		return
	}
	w := activeWire.Window
	msg := fmt.Sprintf("Remove variable %q?", name)
	dlg := dialog.NewConfirm("Remove variable", msg, func(ok bool) {
		if !ok {
			return
		}
		p.macro.RemoveVariable(name)
		p.saveMacro()
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

// RefreshDefs reloads variable definitions from the macro and rebuilds the list.
func (p *VariablesPanel) RefreshDefs() {
	if p.macro == nil {
		p.defs = nil
	} else {
		p.defs = p.macro.CollectVariableDefs()
	}
	if p.list == nil {
		return
	}
	p.list.Refresh()
}

func variablesPanelChrome(panel *VariablesPanel, m *models.Macro) fyne.CanvasObject {
	addBtn := widget.NewButtonWithIcon("Add variable", theme.ContentAddIcon(), func() {
		if m == nil {
			return
		}
		nameEntry := widget.NewEntry()
		nameEntry.SetPlaceHolder("Variable name")
		typeSelect := widget.NewSelect(variableTypeOptions(), nil)
		typeSelect.SetSelected(string(models.VariableTypeAuto))
		valueEntry := widget.NewEntry()
		valueEntry.SetPlaceHolder("Initial value (optional)")
		descEntry := widget.NewEntry()
		descEntry.SetPlaceHolder("Description (optional)")
		form := widget.NewForm(
			widget.NewFormItem("Name", nameEntry),
			widget.NewFormItem("Type", typeSelect),
			widget.NewFormItem("Initial value", valueEntry),
			widget.NewFormItem("Description", descEntry),
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
			m.UpsertVariable(models.VariableDecl{
				Name:         name,
				Type:         models.ParseVariableType(typeSelect.Selected),
				InitialValue: strings.TrimSpace(valueEntry.Text),
				Description:  strings.TrimSpace(descEntry.Text),
			})
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
