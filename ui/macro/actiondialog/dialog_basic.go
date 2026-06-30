package actiondialog

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"image"
	"image/color"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
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
	timeSlider := ttwidget.NewSlider(0.0, 1000.0)
	if t, ok := action.Time.(int); ok {
		timeSlider.SetValue(float64(t))
	}
	timeSlider.OnChanged = func(f float64) {
		timeEntry.Entry.SetText(fmt.Sprintf("%.0f", f))
		timeEntry.Revalidate()
	}
	timeEntry.OnChanged = func(s string) {
		if val, err := strconv.ParseFloat(s, 64); err == nil {
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

// pointCoordToInt returns an int for preview drawing; literal ints are used, variable refs (string) yield 0.

func pointCoordToInt(v any) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

func createMoveDialogContent(action *actions.Move) (fyne.CanvasObject, func()) {
	tempPoint := action.Point

	pointPreviewImage := canvas.NewImageFromImage(nil)
	pointPreviewImage.FillMode = canvas.ImageFillContain
	pointPreviewImage.SetMinSize(previewMin)

	coordsLabel := ttwidget.NewLabel("")
	coordsLabel.SetToolTip("Current X/Y coordinates for the point (numbers or ${variable} expressions). Pick a saved point below or use the preview.")
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

	updatePreview := func(ref actions.CoordinateRef) {
		if ref.IsEmpty() {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}
		pt, err := services.LookupPoint(ref, config.MainMonitorSizeString)
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		px := pointCoordToInt(pt.X)
		py := pointCoordToInt(pt.Y)

		vb := screen.VirtualBounds()
		if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		defer func() {
			if r := recover(); r != nil {
				services.LogPanicToFile(r, "Action dialog: point preview capture")
				pointPreviewImage.Image = nil
				pointPreviewImage.Refresh()
			}
		}()

		captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
		if err != nil || captureImg == nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		mat, err := gocv.ImageToMatRGB(captureImg)
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}
		defer mat.Close()

		center := image.Point{X: px - vb.Min.X, Y: py - vb.Min.Y}
		redColor := color.RGBA{R: 255, A: 255}

		gocv.Circle(&mat, center, 8, redColor, 2)

		gocv.Line(&mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, redColor, 2)
		gocv.Line(&mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, redColor, 2)

		previewImg, err := mat.ToImage()
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		pointPreviewImage.Image = previewImg
		pointPreviewImage.Refresh()
	}

	pointsSearchbar, pointsAccordion := buildPointsAccordionWithSearchbar(func(ref actions.CoordinateRef) {
		tempPoint = ref
		updateCoordsLabel(tempPoint)
		updatePreview(tempPoint)
	}, tempPoint)

	updateCoordsLabel(tempPoint)
	updatePreview(tempPoint)

	smoothForm := newSmoothMoveForm(
		action.Smooth,
		action.EffectiveSmoothLow(),
		action.EffectiveSmoothHigh(),
		action.EffectiveSmoothDelayMs(),
	)

	previewLbl := ttwidget.NewLabel("Preview")
	previewLbl.SetToolTip("Screen snapshot with a crosshair when X/Y are literal numbers (not variables). Variable coordinates skip preview.")

	smoothSettings := widget.NewForm(smoothForm.formItems()...)

	pointsScroll := scrollWithMin(pointsAccordion, fyne.NewSize(splitPanelMinW, previewMinH))

	content := container.NewVBox(
		container.NewHBox(coordsLabel, layout.NewSpacer(), smoothForm.Check),
		smoothSettings,
		container.NewHSplit(
			container.NewBorder(pointsSearchbar, nil, nil, nil, pointsScroll),
			container.NewBorder(previewLbl, nil, nil, nil, pointPreviewImage),
		),
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
	keySelect := ttwidget.NewSelect([]string{"ctrl", "alt", "shift", "win"}, nil)
	keySelect.SetSelected(action.Key)
	keySelect.SetToolTip("Which modifier key to press or release (combined with up/down).")
	wToggle := custom_widgets.NewToggle(func(b bool) {})
	wToggle.SetToggled(action.State)
	upKLbl := ttwidget.NewLabel("up")
	upKLbl.SetToolTip("Release the modifier (key-up).")
	downKLbl := ttwidget.NewLabel("down")
	downKLbl.SetToolTip("Press the modifier (key-down).")

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			keySelect,
			upKLbl,
			wToggle,
			downKLbl,
			layout.NewSpacer(),
		),
	)

	saveFunc := func() {
		action.Key = keySelect.Selected
		action.State = wToggle.Toggled
	}

	return content, saveFunc
}

func createTypeDialogContent(action *actions.Type) (fyne.CanvasObject, func()) {
	textEntry := newReferenceVarEntry()
	textEntry.Entry.SetText(action.Text)
	textEntry.Entry.SetPlaceHolder("Text to type (supports ${variable})")

	delayEntry := widget.NewEntry()
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
	nameEntry := widget.NewEntry()
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
	clausesScroll.SetMinSize(fyne.NewSize(wideFormMinW-160, scrollAreaMinH))

	content := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this conditional in the tree. Used for readability and logging."),
		formHint("Match:", matchSelect, "How to combine clauses: all (AND) requires every clause to be true; any (OR) requires at least one."),
	)
	content.Append("Clauses", container.NewBorder(
		addBtn,
		nil, nil, nil,
		clausesScroll,
	))

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
	nameEntry := widget.NewEntry()
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
