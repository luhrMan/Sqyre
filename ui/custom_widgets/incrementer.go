package custom_widgets

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// halfHeightLayout constrains a single child to half the allocated height (and reports half min height).
type halfHeightLayout struct{}

func (halfHeightLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	if len(objects) == 0 {
		return
	}
	// Pass full allocated size through; half-height is applied only in MinSize.
	objects[0].Resize(fyne.NewSize(size.Width, size.Height))
	objects[0].Move(fyne.NewPos(0, 0))
}

func (halfHeightLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	if len(objects) == 0 {
		return fyne.NewSize(0, 0)
	}
	min := objects[0].MinSize()
	return fyne.NewSize(min.Width, min.Height/2)
}

// Incrementer is a compact widget that displays an integer value with two stacked
// buttons (up/down) to increment or decrement by Step.
type Incrementer struct {
	widget.BaseWidget

	Value    int
	Step     int  // amount to add/subtract per button press; must be > 0
	Min, Max *int // optional bounds; nil means no limit

	OnChanged func(int)

	label   *widget.Label
	upBtn   *widget.Button
	downBtn *widget.Button
}

// NewIncrementer creates an incrementer with the given initial value and step.
// step must be > 0; min and max may be nil for no lower/upper bound.
func NewIncrementer(value int, step int, min, max *int) *Incrementer {
	if step <= 0 {
		step = 1
	}
	inc := &Incrementer{Value: value, Step: step, Min: min, Max: max}
	inc.ExtendBaseWidget(inc)
	inc.label = widget.NewLabel("")
	inc.upBtn = widget.NewButtonWithIcon("", theme.MenuDropUpIcon(), func() { inc.adjust(1) })
	inc.downBtn = widget.NewButtonWithIcon("", theme.MenuDropDownIcon(), func() { inc.adjust(-1) })
	inc.updateLabel()
	return inc
}

func (inc *Incrementer) adjust(delta int) {
	step := inc.Step
	if step <= 0 {
		step = 1
	}
	newVal := inc.Value + delta*step
	if inc.Min != nil && newVal < *inc.Min {
		newVal = *inc.Min
	}
	if inc.Max != nil && newVal > *inc.Max {
		newVal = *inc.Max
	}
	if newVal == inc.Value {
		return
	}
	inc.Value = newVal
	inc.updateLabel()
	if inc.OnChanged != nil {
		inc.OnChanged(inc.Value)
	}
	inc.Refresh()
}

func (inc *Incrementer) updateLabel() {
	inc.label.SetText(fmt.Sprintf("%d", inc.Value))
}

// SetValue sets the value and refreshes the display. It clamps to Min/Max if set.
func (inc *Incrementer) SetValue(v int) {
	if inc.Min != nil && v < *inc.Min {
		v = *inc.Min
	}
	if inc.Max != nil && v > *inc.Max {
		v = *inc.Max
	}
	inc.Value = v
	inc.updateLabel()
	inc.Refresh()
}

// CreateRenderer builds the layout: value label on the left, two stacked buttons on the right (at half height).
func (inc *Incrementer) CreateRenderer() fyne.WidgetRenderer {
	inc.upBtn.Importance = widget.LowImportance
	inc.downBtn.Importance = widget.LowImportance
	buttons := container.NewGridWithRows(2, inc.upBtn, inc.downBtn)
	buttonsHalf := container.New(&halfHeightLayout{}, buttons)
	content := container.NewBorder(nil, nil, nil, buttonsHalf, inc.label)
	return widget.NewSimpleRenderer(content)
}
