package actiondialog

import (
	"Sqyre/internal/models/actions"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
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

	updatePreview := func(point *actions.Point) {
		refreshMovePointPreview(pointPreviewImage, point)
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
