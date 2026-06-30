package actiondialog

import (
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
	timeEntry := newVarEntry()
	timeEntry.SetText(fmt.Sprintf("%d", action.Time))
	timeEntry.SetPlaceHolder("Milliseconds (or ${variable})")
	timeSlider := ttwidget.NewSlider(0.0, 1000.0)
	timeSlider.SetValue(float64(action.Time))
	timeSlider.OnChanged = func(f float64) {
		timeEntry.SetText(fmt.Sprintf("%.0f", f))
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
		if val, err := strconv.Atoi(timeEntry.Text); err == nil {
			action.Time = val
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
	// Temporary storage for the selected point (only applied on save)
	tempPoint := action.Point

	// Create preview image for point preview
	pointPreviewImage := canvas.NewImageFromImage(nil)
	pointPreviewImage.FillMode = canvas.ImageFillContain
	pointPreviewImage.SetMinSize(fyne.NewSize(400, 300))

	// Label showing X and Y expression for the selected point
	coordsLabel := ttwidget.NewLabel(fmt.Sprintf("X: %v, Y: %v", tempPoint.X, tempPoint.Y))
	coordsLabel.SetToolTip("Current X/Y coordinates for the point (numbers or ${variable} expressions). Pick a saved point below or use the preview.")
	updateCoordsLabel := func(point *actions.Point) {
		if point != nil {
			coordsLabel.SetText(fmt.Sprintf("X: %v, Y: %v", point.X, point.Y))
		}
	}

	// Helper function to update preview image (uses pointCoordToInt so variable refs show no marker)
	updatePreview := func(point *actions.Point) {
		if point == nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		px := pointCoordToInt(point.X)
		py := pointCoordToInt(point.Y)

		vb := screen.VirtualBounds()
		if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		// Attempt to capture the full screen with error recovery
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

		// Convert to gocv Mat for drawing
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

		// Convert back to image.Image
		previewImg, err := mat.ToImage()
		if err != nil {
			pointPreviewImage.Image = nil
			pointPreviewImage.Refresh()
			return
		}

		// Update preview image
		pointPreviewImage.Image = previewImg
		pointPreviewImage.Refresh()
	}

	// Points accordion with searchbar above (fuzzy match program name + point name)
	pointsSearchbar, pointsAccordion := buildPointsAccordionWithSearchbar(func(pt actions.Point) {
		tempPoint = pt
		updateCoordsLabel(&tempPoint)
		updatePreview(&tempPoint)
	})

	// Update label and preview for initial point
	updateCoordsLabel(&tempPoint)
	updatePreview(&tempPoint)

	smoothCheck := ttwidget.NewCheck("Smooth", nil)
	smoothCheck.SetChecked(action.Smooth)
	smoothCheck.SetToolTip("When enabled, the mouse moves along a smooth path to the target. When disabled, the cursor jumps instantly.")

	previewLbl := ttwidget.NewLabel("Preview")
	previewLbl.SetToolTip("Screen snapshot with a crosshair when X/Y are literal numbers (not variables). Variable coordinates skip preview.")

	content := container.NewVBox(
		container.NewHBox(coordsLabel, layout.NewSpacer(), smoothCheck),
		container.NewHSplit(
			container.NewBorder(pointsSearchbar, nil, nil, nil, pointsAccordion),
			container.NewBorder(previewLbl, nil, nil, nil, pointPreviewImage),
		),
	)

	saveFunc := func() {
		action.Point = tempPoint
		action.Smooth = smoothCheck.Checked
	}

	return content, saveFunc
}

func createClickDialogContent(action *actions.Click) (fyne.CanvasObject, func()) {
	buttonCheck := custom_widgets.NewToggle(func(b bool) {})
	buttonCheck.SetToggled(action.Button)
	stateToggle := custom_widgets.NewToggle(func(b bool) {})
	stateToggle.SetToggled(action.State)

	leftLbl := ttwidget.NewLabel("left")
	leftLbl.SetToolTip("Toggle toward this side to use the left mouse button.")
	rightLbl := ttwidget.NewLabel("right")
	rightLbl.SetToolTip("Toggle toward this side to use the right mouse button.")
	upLbl := ttwidget.NewLabel("up")
	upLbl.SetToolTip("Toggle toward up to release the button (button-up).")
	downLbl := ttwidget.NewLabel("down")
	downLbl.SetToolTip("Toggle toward down to press the button (button-down).")

	content := container.NewVBox(
		container.NewHBox(
			layout.NewSpacer(),
			leftLbl,
			buttonCheck,
			rightLbl,
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
		action.Button = buttonCheck.Toggled
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
	textEntry := newVarEntry()
	textEntry.SetText(action.Text)
	textEntry.SetPlaceHolder("Text to type (supports ${variable})")

	delayEntry := widget.NewEntry()
	delayEntry.SetText(fmt.Sprintf("%d", action.DelayMs))
	delayEntry.SetPlaceHolder("Delay between key presses (ms)")

	content := widget.NewForm(
		formHint("Text to type:", textEntry, "Characters to send as keystrokes. Use ${variable} for substitution from the macro."),
		formHint("Delay (ms):", delayEntry, "Pause after each character (milliseconds). Use 0 for fastest typing; higher values look more human."),
	)

	saveFunc := func() {
		action.Text = textEntry.Text
		if val, err := strconv.Atoi(strings.TrimSpace(delayEntry.Text)); err == nil && val >= 0 {
			action.DelayMs = val
		}
	}

	return content, saveFunc
}

func createConditionalDialogContent(action *actions.Conditional) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)

	leftEntry := newVarEntry()
	leftEntry.SetPlaceHolder("e.g. ${score} or 10")
	leftEntry.SetText(operandToString(action.Left))

	rightEntry := newVarEntry()
	rightEntry.SetPlaceHolder("e.g. ${target} or 100")
	rightEntry.SetText(operandToString(action.Right))

	operatorSelect := widget.NewSelect(actions.ConditionalOperators, nil)
	if action.Operator != "" {
		operatorSelect.SetSelected(action.Operator)
	} else {
		operatorSelect.SetSelected(actions.OpEquals)
	}

	updateRightState := func(op string) {
		if actions.OperatorIsUnary(op) {
			rightEntry.Disable()
		} else {
			rightEntry.Enable()
		}
	}
	operatorSelect.OnChanged = updateRightState
	updateRightState(operatorSelect.Selected)

	content := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this conditional in the tree. Used for readability and logging."),
		formHint("If (left):", leftEntry, "Left side of the comparison. Literal or ${variable}."),
		formHint("Operator:", operatorSelect, "Comparison operator. Numbers compare numerically; otherwise text is compared. 'is set' / 'is empty' only use the left value."),
		formHint("Value (right):", rightEntry, "Right side of the comparison. Literal or ${variable}. Ignored for 'is set' / 'is empty'."),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		if operatorSelect.Selected != "" {
			action.Operator = operatorSelect.Selected
		} else {
			action.Operator = actions.OpEquals
		}
		action.Left = parseOperand(leftEntry.Text)
		action.Right = parseOperand(rightEntry.Text)
	}

	return content, saveFunc
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
	countEntry := newVarEntry()
	countEntry.SetPlaceHolder("e.g. 5 or ${countVar}")
	switch c := action.Count.(type) {
	case int:
		countEntry.SetText(fmt.Sprintf("%d", c))
	case string:
		countEntry.SetText(c)
	default:
		countEntry.SetText(fmt.Sprintf("%v", c))
	}

	content := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this loop in the tree. Used for readability and logging."),
		formHint("Loops (number or variable):", countEntry, "How many times to run the child actions. Integer or ${variable}. Must be at least 1."),
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		s := strings.TrimSpace(countEntry.Text)
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
