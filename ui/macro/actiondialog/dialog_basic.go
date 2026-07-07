package actiondialog

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)


func createWaitDialogContent(action *actions.Wait) (fyne.CanvasObject, func()) {
	timeEntry := newValidatedVarEntry(validateNumericExpression)
	timeEntry.Entry.SetPlaceHolder("Milliseconds (or ${variable})")
	switch t := action.Time.(type) {
	case int:
		timeEntry.Entry.SetText(fmt.Sprintf("%d", t))
	case string:
		timeEntry.Entry.SetText(t)
	default:
		timeEntry.Entry.SetText(fmt.Sprintf("%v", action.Time))
	}
	const waitSliderMax = 1000.0
	timeSlider := ttwidget.NewSlider(0.0, waitSliderMax)
	if t, ok := action.Time.(int); ok && t >= 0 && t <= int(waitSliderMax) {
		timeSlider.SetValue(float64(t))
	} else if t, ok := action.Time.(int); ok && t > int(waitSliderMax) {
		timeSlider.SetValue(waitSliderMax)
	}
	timeSlider.OnChanged = func(f float64) {
		// Do not push slider values into the entry while it is focused; that
		// races with typing and overwrites partial input.
		if timeEntry.Entry.HasFocus() {
			return
		}
		text := fmt.Sprintf("%.0f", f)
		if timeEntry.Entry.Text != text {
			timeEntry.Entry.SetText(text)
			timeEntry.Revalidate()
		}
	}
	timeEntry.OnChanged = func(s string) {
		val, err := strconv.ParseFloat(strings.TrimSpace(s), 64)
		if err != nil || val < 0 || val > waitSliderMax {
			return
		}
		if timeSlider.Value != val {
			timeSlider.SetValue(val)
		}
	}

	content := widget.NewForm(
		formHint("Duration (ms)", container.NewGridWithColumns(2,
			timeEntry, timeSlider,
		), "How long to block before continuing. Use a number or ${variable}. Typical values are 50–2000 ms."),
	)

	saveFunc := func() {
		s := strings.TrimSpace(timeEntry.Entry.Text)
		if s == "" {
			action.Time = 0
			return
		}
		if val, err := strconv.Atoi(s); err == nil {
			action.Time = val
		} else {
			action.Time = s
		}
	}

	return content, saveFunc
}

func createMoveDialogContent(action *actions.Move) (fyne.CanvasObject, func()) {
	tempPoint := action.Point

	coordsLabel := ttwidget.NewLabel("")
	coordsLabel.SetToolTip("Current X/Y coordinates for the point (numbers or ${variable} expressions). Pick a saved point below.")
	updateCoordsLabel := func(ref actions.CoordinateRef) {
		if ref.IsEmpty() {
			coordsLabel.SetText("No point selected")
			return
		}
		pt, err := services.LookupPoint(ref, config.MainMonitorSizeString)
		if err != nil {
			coordsLabel.SetText(ref.DisplayLabel())
			return
		}
		coordsLabel.SetText(fmt.Sprintf("%s — X: %v, Y: %v", ref.DisplayLabel(), pt.X, pt.Y))
	}

	pointsSearchbar, pointsList := buildPointsListWithSearchbar(func(ref actions.CoordinateRef) {
		tempPoint = ref
		updateCoordsLabel(tempPoint)
	}, tempPoint)

	updateCoordsLabel(tempPoint)

	smoothForm := newSmoothMoveForm(
		action.Smooth,
		action.EffectiveSmoothLow(),
		action.EffectiveSmoothHigh(),
		action.EffectiveSmoothDelayMs(),
	)

	smoothSettings := widget.NewForm(smoothForm.formItems()...)

	pointsScroll := scrollWithMinW(pointsList, splitPanelMinW)

	content := container.NewBorder(
		container.NewVBox(
			container.NewHBox(coordsLabel, layout.NewSpacer(), smoothForm.Check),
			smoothSettings,
		),
		nil, nil, nil,
		container.NewBorder(pointsSearchbar, nil, nil, nil, pointsScroll),
	)

	saveFunc := func() {
		action.Point = tempPoint
		var low float64
		var high float64
		var delayMs int
		smoothForm.writeTo(&action.Smooth, &low, &high, &delayMs)
		if action.Smooth {
			action.SmoothLow = low
			action.SmoothHigh = high
			action.SmoothDelayMs = delayMs
		}
	}

	return content, saveFunc
}

func createClickDialogContent(action *actions.Click) (fyne.CanvasObject, func()) {
	buttonSelect := ttwidget.NewSelect(actions.ClickButtons, nil)
	buttonSelect.SetSelected(action.Button)
	if buttonSelect.Selected == "" {
		buttonSelect.SetSelected(actions.ClickButtonLeft)
	}
	buttonSelect.SetToolTip("Which mouse button or scroll wheel to act on.")

	stateToggle := custom_widgets.NewToggle(func(b bool) {})
	stateToggle.SetToggled(action.State)

	upLbl := ttwidget.NewLabel("up")
	upLbl.SetToolTip("Release the button (button-up), or scroll up when scroll is selected.")
	downLbl := ttwidget.NewLabel("down")
	downLbl.SetToolTip("Press the button (button-down), or scroll down when scroll is selected.")

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			buttonSelect,
			layout.NewSpacer(),
		),
		container.NewHBox(
			layout.NewSpacer(),
			upLbl,
			stateToggle,
			downLbl,
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.Button = buttonSelect.Selected
		action.State = stateToggle.Toggled
	}

	return content, saveFunc
}

func createKeyDialogContent(action *actions.Key) (fyne.CanvasObject, func()) {
	keyLabel := widget.NewLabel(action.Key)
	if keyLabel.Text == "" {
		keyLabel.SetText("(not set)")
	}
	keyLabel.TextStyle = fyne.TextStyle{Monospace: true}

	recordBtn := widget.NewButtonWithIcon("Record key", theme.MediaRecordIcon(), func() {
		if active.ShowKeyRecordDialog == nil || active.Window == nil {
			return
		}
		active.ShowKeyRecordDialog(active.Window, func(key string) {
			action.Key = key
			keyLabel.SetText(key)
			if keyLabel.Text == "" {
				keyLabel.SetText("(not set)")
			}
			notifyDialogValidationChanged()
		})
	})

	wToggle := custom_widgets.NewToggle(func(b bool) {})
	wToggle.SetToggled(action.State)
	upKLbl := ttwidget.NewLabel("up")
	upKLbl.SetToolTip("Release the key (key-up).")
	downKLbl := ttwidget.NewLabel("down")
	downKLbl.SetToolTip("Press the key (key-down).")

	trackDialogValidityCheck(func() bool {
		return strings.TrimSpace(action.Key) != ""
	})

	content := container.NewVBox(
		widget.NewForm(
			formHint("Key:", container.NewVBox(keyLabel, recordBtn),
				"Press Record key and hold the key you want. Letters, numbers, function keys, arrows, modifiers, and Escape are supported."),
		),
		container.NewHBox(
			layout.NewSpacer(),
			upKLbl,
			wToggle,
			downKLbl,
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.State = wToggle.Toggled
	}

	return content, saveFunc
}

func createTypeDialogContent(action *actions.Type) (fyne.CanvasObject, func()) {
	textEntry := newReferenceVarEntry()
	textEntry.Entry.SetText(action.Text)
	textEntry.Entry.SetPlaceHolder("Text to type (supports ${variable})")

	delayEntry := custom_widgets.NewFormEntry()
	delayEntry.SetText(fmt.Sprintf("%d", action.DelayMs))
	delayEntry.SetPlaceHolder("Delay between key presses (ms)")

	content := widget.NewForm(
		formHint("Text to type:", textEntry, "Characters to send as keystrokes. Use ${variable} for substitution from the macro."),
		formHint("Delay (ms):", delayEntry, "Pause after each character (milliseconds). Use 0 for fastest typing; higher values look more human."),
	)

	saveFunc := func() {
		action.Text = textEntry.Entry.Text
		if val, err := strconv.Atoi(strings.TrimSpace(delayEntry.Text)); err == nil && val >= 0 {
			action.DelayMs = val
		}
	}

	return content, saveFunc
}

func createConditionalDialogContent(action *actions.Conditional) (fyne.CanvasObject, func()) {
	nameEntry := custom_widgets.NewFormEntry()
	nameEntry.SetText(action.Name)

	matchSelect := widget.NewSelect(actions.ConditionalMatchModes, nil)
	matchSelect.SetSelected(action.EffectiveMatch())

	rows := make([]clauseRowWidgets, len(action.Clauses))
	for i, c := range action.Clauses {
		rows[i] = newClauseRowWidgets(c)
	}
	if len(rows) == 0 {
		rows = append(rows, newClauseRowWidgets(actions.ConditionClause{}))
	}

	rowsBox := container.NewVBox()
	var rebuild func()
	rebuild = func() {
		rowsBox.Objects = nil
		for i := range rows {
			idx := i
			row := rows[idx]
			removeBtn := widget.NewButton("Remove", func() {
				if len(rows) <= 1 {
					return
				}
				rows = append(rows[:idx], rows[idx+1:]...)
				rebuild()
			})
			if len(rows) <= 1 {
				removeBtn.Disable()
			}
			header := widget.NewLabelWithStyle(fmt.Sprintf("Clause %d", idx+1), fyne.TextAlignLeading, fyne.TextStyle{Bold: true})
			rowForm := widget.NewForm(
				formHint("If (left):", row.left, "Left side of the comparison. Literal or ${variable}."),
				formHint("Operator:", row.operator, "Comparison operator. Numbers compare numerically; otherwise text is compared. 'is set' / 'is empty' only use the left value."),
				formHint("Value (right):", row.right, "Right side of the comparison. Literal or ${variable}. Ignored for 'is set' / 'is empty'."),
			)
			rowsBox.Add(container.NewVBox(
				header,
				rowForm,
				removeBtn,
				widget.NewSeparator(),
			))
		}
	}
	rebuild()

	addBtn := widget.NewButton("Add clause", func() {
		rows = append(rows, newClauseRowWidgets(actions.ConditionClause{}))
		rebuild()
	})

	clausesScroll := container.NewVScroll(rowsBox)
	clausesScroll.SetMinSize(fyne.NewSize(wideFormMinW-160, 0))

	headerForm := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this conditional in the tree. Used for readability and logging."),
		formHint("Match:", matchSelect, "How to combine clauses: all (AND) requires every clause to be true; any (OR) requires at least one."),
	)
	clausesLabel := widget.NewLabelWithStyle("Clauses", fyne.TextAlignLeading, fyne.TextStyle{Bold: true})
	clausesPanel := container.NewBorder(
		container.NewVBox(clausesLabel, addBtn),
		nil, nil, nil,
		clausesScroll,
	)
	content := container.NewBorder(headerForm, nil, nil, nil, clausesPanel)

	saveFunc := func() {
		action.Name = nameEntry.Text
		if matchSelect.Selected != "" {
			action.Match = matchSelect.Selected
		} else {
			action.Match = actions.MatchAll
		}
		clauses := make([]actions.ConditionClause, 0, len(rows))
		for _, row := range rows {
			op := row.operator.Selected
			if op == "" {
				op = actions.OpEquals
			}
			clauses = append(clauses, actions.ConditionClause{
				Left:     parseOperand(row.left.Entry.Text),
				Operator: op,
				Right:    parseOperand(row.right.Entry.Text),
			})
		}
		action.Clauses = clauses
	}

	return content, saveFunc
}

type clauseRowWidgets struct {
	left     *custom_widgets.VarEntryField
	operator *widget.Select
	right    *custom_widgets.VarEntryField
}

func newClauseRowWidgets(c actions.ConditionClause) clauseRowWidgets {
	left := newReferenceVarEntry()
	left.Entry.SetPlaceHolder("e.g. ${score} or 10")
	left.Entry.SetText(operandToString(c.Left))

	right := newReferenceVarEntry()
	right.Entry.SetPlaceHolder("e.g. ${target} or 100")
	right.Entry.SetText(operandToString(c.Right))

	operatorSelect := widget.NewSelect(actions.ConditionalOperators, nil)
	if c.Operator != "" {
		operatorSelect.SetSelected(c.Operator)
	} else {
		operatorSelect.SetSelected(actions.OpEquals)
	}

	updateRightState := func(op string) {
		if actions.OperatorIsUnary(op) {
			right.Entry.Disable()
		} else {
			right.Entry.Enable()
		}
	}
	operatorSelect.OnChanged = updateRightState
	updateRightState(operatorSelect.Selected)

	return clauseRowWidgets{
		left:     left,
		operator: operatorSelect,
		right:    right,
	}
}

// operandToString renders a conditional operand (int or string) for an entry field.
func operandToString(v any) string {
	switch val := v.(type) {
	case nil:
		return ""
	case string:
		return val
	case int:
		return strconv.Itoa(val)
	default:
		return fmt.Sprintf("%v", val)
	}
}

// parseOperand converts entry text to an int literal when possible, else keeps the string.
func parseOperand(text string) any {
	s := strings.TrimSpace(text)
	if s == "" {
		return ""
	}
	if n, err := strconv.Atoi(s); err == nil {
		return n
	}
	return s
}

func createLoopDialogContent(action *actions.Loop) (fyne.CanvasObject, func()) {
	nameEntry := custom_widgets.NewFormEntry()
	nameEntry.SetText(action.Name)
	countEntry := newValidatedVarEntry(validateNumericExpression)
	countEntry.Entry.SetPlaceHolder("e.g. 5 or ${countVar}")
	switch c := action.Count.(type) {
	case int:
		countEntry.Entry.SetText(fmt.Sprintf("%d", c))
	case string:
		countEntry.Entry.SetText(c)
	default:
		countEntry.Entry.SetText(fmt.Sprintf("%v", c))
	}

	content := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this loop in the tree. Used for readability and logging."),
		formHint("Loops (number or variable):", countEntry, "How many times to run the child actions. Integer or ${variable}. Must be at least 1."),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		s := strings.TrimSpace(countEntry.Entry.Text)
		if s == "" {
			action.Count = 1
			return
		}
		if count, err := strconv.Atoi(s); err == nil {
			action.Count = count
		} else {
			action.Count = s
		}
	}

	return content, saveFunc
}
