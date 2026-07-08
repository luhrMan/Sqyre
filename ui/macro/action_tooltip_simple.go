package macro

import (
	"fmt"
	"strconv"
	"strings"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func parseNumericOrVar(text string) any {
	text = strings.TrimSpace(text)
	if text == "" {
		return ""
	}
	if strings.HasPrefix(text, "${") {
		return text
	}
	if i, err := strconv.Atoi(text); err == nil {
		return i
	}
	return text
}

func appendClickTooltipView(a *actions.Click, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	row.add(actiondisplay.NewDisplaySelectPill("Button", a.Button, actions.ClickButtonLabel, actionType))
	row.add(actiondisplay.NewDisplayTogglePill("State down", a.State, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendClickTooltipEdit(a *actions.Click, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	button := a.Button
	if button == "" {
		button = actions.ClickButtonLeft
	}
	buttonSelect := actiondisplay.NewPillSelect("Button", actions.ClickButtons, button, actions.ClickButtonLabel)
	stateToggle := actiondisplay.NewPillToggle("State down", a.State)
	row.add(actiondisplay.WrapPillSelect(buttonSelect, actionType))
	row.add(actiondisplay.WrapPillToggle(stateToggle, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.Button = buttonSelect.Value
		a.State = stateToggle.Value
		return nil
	}
}

func appendKeyTooltipView(a *actions.Key, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Key", a.Key, actionType)
	row.add(actiondisplay.NewDisplayTogglePill("State down", a.State, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendKeyTooltipEdit(a *actions.Key, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	keyEntry := coordEntry(a.Key)
	stateToggle := actiondisplay.NewPillToggle("State down", a.State)
	row.add(actiondisplay.NewEditablePill("Key", keyEntry, actionType))
	if activeWire.ShowKeyRecordDialog != nil && activeWire.Window != nil {
		recordBtn := actiondisplay.NewPillIconButton(theme.MediaRecordIcon(), func() {
			activeWire.ShowKeyRecordDialog(activeWire.Window, func(key string) {
				keyEntry.SetText(key)
			})
		})
		row.add(actiondisplay.PillChrome(recordBtn, actionType))
	}
	row.add(actiondisplay.WrapPillToggle(stateToggle, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.Key = strings.TrimSpace(keyEntry.Text)
		a.State = stateToggle.Value
		return nil
	}
}

func appendWaitTooltipView(a *actions.Wait, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Time", formatAnyValue(a.Time), actionType)
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendWaitTooltipEdit(a *actions.Wait, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	timeEntry := coordEntry(formatAnyValue(a.Time))
	row.add(actiondisplay.NewEditablePill("Time (ms)", timeEntry, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		s := strings.TrimSpace(timeEntry.Text)
		if s == "" {
			a.Time = 0
			return nil
		}
		if val, err := strconv.Atoi(s); err == nil {
			a.Time = val
		} else {
			a.Time = s
		}
		return nil
	}
}

func appendLoopTooltipView(a *actions.Loop, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Name", a.Name, actionType)
	addDisplayPill(row, "Iterations", formatAnyValue(a.Count), actionType)
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendLoopTooltipEdit(a *actions.Loop, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	nameEntry := coordEntry(a.Name)
	countEntry := coordEntry(formatAnyValue(a.Count))
	row.add(actiondisplay.NewEditablePill("Name", nameEntry, actionType))
	row.add(actiondisplay.NewEditablePill("Iterations", countEntry, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.Name = strings.TrimSpace(nameEntry.Text)
		s := strings.TrimSpace(countEntry.Text)
		if s == "" {
			a.Count = 1
			return nil
		}
		if count, err := strconv.Atoi(s); err == nil {
			a.Count = count
		} else {
			a.Count = s
		}
		return nil
	}
}

func appendConditionalTooltipView(a *actions.Conditional, actionType string) []fyne.CanvasObject {
	var sections []fyne.CanvasObject

	general := newPillRow()
	addDisplayPill(general, "Name", a.Name, actionType)
	isMatchAny := a.EffectiveMatch() == actions.MatchAny
	general.add(actiondisplay.NewDisplayTogglePill(conditionalMatchToggleLabel(isMatchAny), isMatchAny, actionType))
	sections = append(sections, wrapTooltipSection(general.box))

	if len(a.Clauses) == 0 {
		sections = append(sections, conditionalClauseViewPill(1, true, actions.ConditionClause{Operator: actions.OpEquals}, actionType))
		return sections
	}
	for i, c := range a.Clauses {
		sections = append(sections, conditionalClauseViewPill(i+1, false, c, actionType))
	}
	return sections
}

func conditionalClauseViewPill(clauseNum int, shortIfLabel bool, c actions.ConditionClause, actionType string) fyne.CanvasObject {
	op := c.Operator
	if op == "" {
		op = actions.OpEquals
	}
	clauseLabel := fmt.Sprintf("%d If", clauseNum)
	if shortIfLabel {
		clauseLabel = "If"
	}
	row := newPillRow()
	addDisplayPill(row, clauseLabel, formatAnyValue(c.Left), actionType)
	row.add(actiondisplay.NewDisplayDropdownPill("op", op, nil, actionType))
	if !actions.OperatorIsUnary(op) {
		if right := strings.TrimSpace(formatAnyValue(c.Right)); right != "" {
			row.add(actiondisplay.NewDisplayValuePill(right, actionType, macroKnownVariables()))
		}
	}
	return wrapTooltipSection(row.box)
}

func conditionalMatchToggleLabel(matchAny bool) string {
	if matchAny {
		return "Match any (OR)"
	}
	return "Match all (AND)"
}

type conditionalClauseEditors struct {
	left     *custom_widgets.BorderlessEntry
	operator *actiondisplay.PillDropdown
	right    *custom_widgets.BorderlessEntry
}

func wireConditionalClauseRight(entry *custom_widgets.BorderlessEntry, opSelect *actiondisplay.PillDropdown) {
	setRightEnabled := func(op string) {
		if actions.OperatorIsUnary(op) {
			entry.Disable()
			return
		}
		entry.Enable()
	}
	opSelect.OnChanged = setRightEnabled
	setRightEnabled(opSelect.Value)
}

func appendConditionalTooltipEdit(a *actions.Conditional, actionType string, owner *actionDisplayTooltipHover) ([]fyne.CanvasObject, func() error) {
	clauseEditors := make([]conditionalClauseEditors, len(a.Clauses))
	for i, c := range a.Clauses {
		op := c.Operator
		if op == "" {
			op = actions.OpEquals
		}
		clauseEditors[i] = conditionalClauseEditors{
			left:     coordEntry(formatAnyValue(c.Left)),
			operator: actiondisplay.NewPillDropdown("op", actions.ConditionalOperators, op, nil),
			right:    coordEntry(formatAnyValue(c.Right)),
		}
		wireConditionalClauseRight(clauseEditors[i].right, clauseEditors[i].operator)
	}
	if len(clauseEditors) == 0 {
		opSelect := actiondisplay.NewPillDropdown("op", actions.ConditionalOperators, actions.OpEquals, nil)
		rightEntry := coordEntry("")
		wireConditionalClauseRight(rightEntry, opSelect)
		clauseEditors = append(clauseEditors, conditionalClauseEditors{
			left:     coordEntry(""),
			operator: opSelect,
			right:    rightEntry,
		})
	}

	var sections []fyne.CanvasObject
	general := newPillRow()
	nameEntry := coordEntry(a.Name)
	isMatchAny := a.EffectiveMatch() == actions.MatchAny
	matchToggle := actiondisplay.NewPillToggle(conditionalMatchToggleLabel(isMatchAny), isMatchAny)
	matchToggle.OnChanged = func(matchAny bool) {
		matchToggle.SetLabel(conditionalMatchToggleLabel(matchAny))
	}
	general.add(actiondisplay.NewEditablePill("Name", nameEntry, actionType))
	general.add(actiondisplay.WrapPillToggle(matchToggle, actionType))
	sections = append(sections, wrapTooltipSection(general.box))

	clausesBox := container.NewVBox()
	var rebuildClauses func()
	rebuildClauses = func() {
		clausesBox.Objects = nil
		for i := range clauseEditors {
			idx := i
			clause := newPillRow()
			label := fmt.Sprintf("%d If", idx+1)
			if len(clauseEditors) == 1 {
				label = "If"
			}
			clause.add(actiondisplay.NewEditablePill(label, clauseEditors[idx].left, actionType))
			clause.add(actiondisplay.WrapPillDropdown(clauseEditors[idx].operator, actionType))
			clause.add(actiondisplay.PillChrome(clauseEditors[idx].right, actionType))
			if len(clauseEditors) > 1 {
				removeIdx := idx
				clause.add(actiondisplay.PillChrome(actiondisplay.NewPillIconButton(theme.CancelIcon(), func() {
					clauseEditors = append(clauseEditors[:removeIdx], clauseEditors[removeIdx+1:]...)
					rebuildClauses()
				}), actionType))
			}
			clausesBox.Add(wrapTooltipSection(clause.box))
		}
		addBtn := widget.NewButton("Add clause", func() {
			opSelect := actiondisplay.NewPillDropdown("op", actions.ConditionalOperators, actions.OpEquals, nil)
			rightEntry := coordEntry("")
			wireConditionalClauseRight(rightEntry, opSelect)
			clauseEditors = append(clauseEditors, conditionalClauseEditors{
				left:     coordEntry(""),
				operator: opSelect,
				right:    rightEntry,
			})
			rebuildClauses()
		})
		clausesBox.Add(wrapTooltipSection(container.NewCenter(addBtn)))
		clausesBox.Refresh()
		if owner != nil {
			owner.relayoutTooltip()
		}
	}
	rebuildClauses()
	sections = append(sections, clausesBox)

	return sections, func() error {
		a.Name = strings.TrimSpace(nameEntry.Text)
		if matchToggle.Value {
			a.Match = actions.MatchAny
		} else {
			a.Match = actions.MatchAll
		}
		clauses := make([]actions.ConditionClause, 0, len(clauseEditors))
		for _, ed := range clauseEditors {
			clauses = append(clauses, actions.ConditionClause{
				Left:     parseNumericOrVar(ed.left.Text),
				Operator: ed.operator.Value,
				Right:    parseNumericOrVar(ed.right.Text),
			})
		}
		a.Clauses = clauses
		return nil
	}
}

func appendSetVariableTooltipView(a *actions.SetVariable, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Variable", a.VariableName, actionType)
	addDisplayPill(row, "Value", formatAnyValue(a.Value), actionType)
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendSetVariableTooltipEdit(a *actions.SetVariable, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	nameEntry := coordEntry(a.VariableName)
	valueEntry := coordEntry(formatAnyValue(a.Value))
	row.add(actiondisplay.NewEditablePill("Variable", nameEntry, actionType))
	row.add(actiondisplay.NewEditablePill("Value", valueEntry, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.VariableName = strings.TrimSpace(nameEntry.Text)
		a.Value = valueEntry.Text
		return nil
	}
}

func appendCalculateTooltipView(a *actions.Calculate, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Expression", a.Expression, actionType)
	addDisplayPill(row, "Output", a.OutputVar, actionType)
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendCalculateTooltipEdit(a *actions.Calculate, actionType string) ([]fyne.CanvasObject, func() error) {
	var sections []fyne.CanvasObject
	exprEntry := coordEntry(a.Expression)
	toolbar := calculateBuilderToolbar(exprEntry)
	sections = append(sections, wrapTooltipSection(toolbar))

	exprRow := newPillRow()
	exprRow.add(actiondisplay.NewEditablePill("Expression", exprEntry, actionType))
	sections = append(sections, wrapTooltipSection(exprRow.box))

	previewSection, _ := appendCalculatePreviewRow(exprEntry, actionType)
	if previewSection != nil {
		sections = append(sections, previewSection)
	}

	outputRow := newPillRow()
	outputEntry := coordEntry(a.OutputVar)
	outputRow.add(actiondisplay.NewEditablePill("Output", outputEntry, actionType))
	sections = append(sections, wrapTooltipSection(outputRow.box))

	return sections, func() error {
		a.Expression = exprEntry.Text
		a.OutputVar = strings.TrimSpace(outputEntry.Text)
		return nil
	}
}

func appendRunMacroTooltipView(a *actions.RunMacro, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	addDisplayPill(row, "Macro", a.MacroName, actionType)
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendRunMacroTooltipEdit(a *actions.RunMacro, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	macroEntry := coordEntry(a.MacroName)
	row.add(actiondisplay.NewEditablePill("Macro", macroEntry, actionType))
	row.add(macroPickerButton(a.MacroName, actionType, func(name string) {
		macroEntry.SetText(name)
	}))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.MacroName = strings.TrimSpace(macroEntry.Text)
		return nil
	}
}

func appendFlowControlTooltipView(actionType, description string) []fyne.CanvasObject {
	row := newPillRow()
	row.add(actiondisplay.NewDisplayPill(description, actionType))
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendMoveTooltipView(a *actions.Move, actionType string) []fyne.CanvasObject {
	row := newPillRow()
	row.add(actiondisplay.NewDisplayTogglePill("Smooth", a.Smooth, actionType))
	if a.Smooth {
		row.add(actiondisplay.NewDisplayPill("Smooth low: "+actions.FormatParamValue(a.EffectiveSmoothLow()), actionType))
		row.add(actiondisplay.NewDisplayPill("Smooth high: "+actions.FormatParamValue(a.EffectiveSmoothHigh()), actionType))
		row.add(actiondisplay.NewDisplayPill("Smooth delay (ms): "+actions.FormatParamValue(a.EffectiveSmoothDelayMs()), actionType))
	}
	return []fyne.CanvasObject{wrapTooltipSection(row.box)}
}

func appendMoveTooltipEdit(a *actions.Move, actionType string) ([]fyne.CanvasObject, func() error) {
	row := newPillRow()
	smoothToggle := actiondisplay.NewPillToggle("Smooth", a.Smooth)
	row.add(actiondisplay.WrapPillToggle(smoothToggle, actionType))

	lowMin, lowMax := 0.0, 1.0
	highMin, highMax := 0.0, 1.0
	delayMin, delayMax := 0, 10000
	lowInc := actiondisplay.NewPillFloatStepper("Smooth low", a.EffectiveSmoothLow(), 0.05, &lowMin, &lowMax, 2, actionType)
	highInc := actiondisplay.NewPillFloatStepper("Smooth high", a.EffectiveSmoothHigh(), 0.05, &highMin, &highMax, 2, actionType)
	delayInc := actiondisplay.NewPillIntStepper("Smooth delay (ms)", a.EffectiveSmoothDelayMs(), 1, &delayMin, &delayMax, actionType)
	row.add(actiondisplay.WrapPillStepper(lowInc, actionType))
	row.add(actiondisplay.WrapPillStepper(highInc, actionType))
	row.add(actiondisplay.WrapPillStepper(delayInc, actionType))

	setSmoothEnabled := func(enabled bool) {
		if enabled {
			lowInc.Enable()
			highInc.Enable()
			delayInc.Enable()
			return
		}
		lowInc.Disable()
		highInc.Disable()
		delayInc.Disable()
	}
	wirePillToggleSection(smoothToggle, setSmoothEnabled)

	return []fyne.CanvasObject{wrapTooltipSection(row.box)}, func() error {
		a.Smooth = smoothToggle.Value
		if a.Smooth {
			a.SmoothLow = lowInc.Value
			a.SmoothHigh = highInc.Value
			a.SmoothDelayMs = delayInc.Value
		}
		return nil
	}
}
