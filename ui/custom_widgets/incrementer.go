package custom_widgets

import (
	"fmt"
	"image/color"
	"strconv"
	"strings"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// borderedIncrementButton draws a light gray rounded border around one increment/decrement control.
func borderedIncrementButton(btn fyne.CanvasObject) fyne.CanvasObject {
	border := canvas.NewRectangle(color.Transparent)
	border.StrokeColor = theme.DisabledColor()
	border.StrokeWidth = 1
	border.CornerRadius = theme.InputRadiusSize()
	return container.NewStack(border, btn)
}

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
//
// Value display is either a Label (NewIncrementer) or an Entry (NewIncrementerWithStepEntry),
// never both.
type Incrementer struct {
	widget.BaseWidget

	Value    int
	Step     int  // amount to add/subtract per button press; must be > 0
	Min, Max *int // optional bounds; nil means no limit

	OnChanged func(int)

	label      *widget.Label
	valueEntry *widget.Entry // when non-nil, replaces label as the value display (editable)
	// suppressValueEntry avoids valueEntry OnChanged when SetText is driven by the incrementer.
	suppressValueEntry bool

	upBtn   *holdRepeatButton
	downBtn *holdRepeatButton
}

// FloatIncrementer is a compact widget that displays a float value with two stacked
// buttons (up/down) to increment or decrement by Step.
type FloatIncrementer struct {
	widget.BaseWidget

	Value     float64
	Step      float64
	Min, Max  *float64
	Precision int

	OnChanged func(float64)

	label   *widget.Label
	upBtn   *holdRepeatButton
	downBtn *holdRepeatButton
}

// holdRepeatButton triggers action once on tap and repeatedly while held.
type holdRepeatButton struct {
	widget.Button

	action func()

	holdMu            sync.Mutex
	holdStop          chan struct{}
	consumeNextTap    bool
	hasRepeatedOnHold bool
}

func newHoldRepeatButton(icon fyne.Resource, action func()) *holdRepeatButton {
	btn := &holdRepeatButton{action: action}
	btn.Text = ""
	btn.Icon = icon
	btn.OnTapped = nil // handled via Tapped override
	btn.ExtendBaseWidget(btn)
	return btn
}

func (b *holdRepeatButton) MouseDown(me *desktop.MouseEvent) {
	if b.Disabled() || me == nil || me.Button != desktop.MouseButtonPrimary {
		return
	}

	b.stopHold(false)

	b.holdMu.Lock()
	stop := make(chan struct{})
	b.holdStop = stop
	b.hasRepeatedOnHold = false
	b.holdMu.Unlock()

	go func() {
		const initialDelay = 350 * time.Millisecond
		const repeatEvery = 100 * time.Millisecond

		timer := time.NewTimer(initialDelay)
		defer timer.Stop()

		select {
		case <-stop:
			return
		case <-timer.C:
		}

		ticker := time.NewTicker(repeatEvery)
		defer ticker.Stop()

		for {
			select {
			case <-stop:
				return
			case <-ticker.C:
				fyne.Do(func() {
					if b.Disabled() {
						return
					}
					b.holdMu.Lock()
					b.hasRepeatedOnHold = true
					b.holdMu.Unlock()
					if b.action != nil {
						b.action()
					}
				})
			}
		}
	}()
}

func (b *holdRepeatButton) MouseUp(me *desktop.MouseEvent) {
	if me != nil && me.Button != desktop.MouseButtonPrimary {
		return
	}
	b.stopHold(true)
}

func (b *holdRepeatButton) MouseOut() {
	b.Button.MouseOut()
	b.stopHold(true)
}

func (b *holdRepeatButton) Tapped(*fyne.PointEvent) {
	if b.Disabled() {
		return
	}

	b.holdMu.Lock()
	consumeTap := b.consumeNextTap
	b.consumeNextTap = false
	b.holdMu.Unlock()

	if consumeTap {
		return
	}
	if b.action != nil {
		b.action()
	}
}

func (b *holdRepeatButton) stopHold(markConsume bool) {
	b.holdMu.Lock()
	stop := b.holdStop
	b.holdStop = nil
	repeated := b.hasRepeatedOnHold
	b.hasRepeatedOnHold = false
	if markConsume {
		b.consumeNextTap = repeated
	}
	b.holdMu.Unlock()

	if stop != nil {
		close(stop)
	}
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
	inc.upBtn = newHoldRepeatButton(theme.MenuDropUpIcon(), func() { inc.adjust(1) })
	inc.downBtn = newHoldRepeatButton(theme.MenuDropDownIcon(), func() { inc.adjust(-1) })
	inc.syncValueDisplay()
	return inc
}

// NewIncrementerWithEntry creates an incrementer with an editable value entry (no label).
func NewIncrementerWithEntry(value int, step int, min, max *int) *Incrementer {
	if step <= 0 {
		step = 1
	}
	inc := &Incrementer{Value: value, Step: step, Min: min, Max: max}
	inc.ExtendBaseWidget(inc)
	ve := widget.NewEntry()
	ve.SetText(strconv.Itoa(value))
	inc.valueEntry = ve
	inc.upBtn = newHoldRepeatButton(theme.MenuDropUpIcon(), func() { inc.adjust(1) })
	inc.downBtn = newHoldRepeatButton(theme.MenuDropDownIcon(), func() { inc.adjust(-1) })

	inc.valueEntry.OnChanged = func(s string) {
		if inc.suppressValueEntry {
			return
		}
		v, err := strconv.Atoi(strings.TrimSpace(s))
		if err != nil {
			return
		}
		if inc.Min != nil && v < *inc.Min {
			v = *inc.Min
			inc.setValueEntryText(strconv.Itoa(v))
			inc.Value = v
			if inc.OnChanged != nil {
				inc.OnChanged(inc.Value)
			}
			return
		}
		if inc.Max != nil && v > *inc.Max {
			v = *inc.Max
			inc.setValueEntryText(strconv.Itoa(v))
			inc.Value = v
			if inc.OnChanged != nil {
				inc.OnChanged(inc.Value)
			}
			return
		}
		if v == inc.Value {
			return
		}
		inc.Value = v
		if inc.OnChanged != nil {
			inc.OnChanged(inc.Value)
		}
	}

	return inc
}

func (inc *Incrementer) setValueEntryText(s string) {
	if inc.valueEntry == nil {
		return
	}
	inc.suppressValueEntry = true
	inc.valueEntry.SetText(s)
	inc.suppressValueEntry = false
}

func (inc *Incrementer) adjust(delta int) {
	step := inc.Step
	if step <= 0 {
		step = 1
	}
	base := inc.Value
	if inc.valueEntry != nil {
		if v, err := strconv.Atoi(strings.TrimSpace(inc.valueEntry.Text)); err == nil {
			base = v
		}
	}
	newVal := base + delta*step
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
	inc.syncValueDisplay()
	if inc.OnChanged != nil {
		inc.OnChanged(inc.Value)
	}
	inc.Refresh()
}

func (inc *Incrementer) syncValueDisplay() {
	if inc.valueEntry != nil {
		inc.setValueEntryText(strconv.Itoa(inc.Value))
		return
	}
	if inc.label != nil {
		inc.label.SetText(fmt.Sprintf("%d", inc.Value))
	}
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
	inc.syncValueDisplay()
	inc.Refresh()
}

// CreateRenderer builds the layout: value label or value entry on the left,
// with two stacked buttons on the right (at half height).
func (inc *Incrementer) CreateRenderer() fyne.WidgetRenderer {
	inc.upBtn.Importance = widget.LowImportance
	inc.downBtn.Importance = widget.LowImportance
	buttons := container.NewGridWithRows(2,
		borderedIncrementButton(inc.upBtn),
		borderedIncrementButton(inc.downBtn),
	)
	buttonsHalf := container.New(&halfHeightLayout{}, buttons)
	var right fyne.CanvasObject = buttonsHalf
	var center fyne.CanvasObject
	if inc.valueEntry != nil {
		center = inc.valueEntry
	} else {
		center = inc.label
	}
	content := container.NewBorder(nil, nil, nil, right, center)
	return widget.NewSimpleRenderer(content)
}

// Disable disables both increment/decrement controls.
func (inc *Incrementer) Disable() {
	inc.upBtn.Disable()
	inc.downBtn.Disable()
	if inc.valueEntry != nil {
		inc.valueEntry.Disable()
	}
	inc.Refresh()
}

// Enable enables both increment/decrement controls.
func (inc *Incrementer) Enable() {
	inc.upBtn.Enable()
	inc.downBtn.Enable()
	if inc.valueEntry != nil {
		inc.valueEntry.Enable()
	}
	inc.Refresh()
}

// NewFloatIncrementer creates an incrementer with the given initial float value and step.
// step must be > 0; min and max may be nil for no lower/upper bound.
func NewFloatIncrementer(value float64, step float64, min, max *float64, precision int) *FloatIncrementer {
	if step <= 0 {
		step = 1
	}
	if precision < 0 {
		precision = 2
	}
	inc := &FloatIncrementer{
		Value:     value,
		Step:      step,
		Min:       min,
		Max:       max,
		Precision: precision,
	}
	inc.ExtendBaseWidget(inc)
	inc.label = widget.NewLabel("")
	inc.upBtn = newHoldRepeatButton(theme.MenuDropUpIcon(), func() { inc.adjust(1) })
	inc.downBtn = newHoldRepeatButton(theme.MenuDropDownIcon(), func() { inc.adjust(-1) })
	inc.SetValue(value)
	return inc
}

func (inc *FloatIncrementer) adjust(delta int) {
	step := inc.Step
	if step <= 0 {
		step = 1
	}
	newVal := inc.Value + float64(delta)*step
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

func (inc *FloatIncrementer) updateLabel() {
	inc.label.SetText(fmt.Sprintf("%.*f", inc.Precision, inc.Value))
}

// SetValue sets the value and refreshes the display. It clamps to Min/Max if set.
func (inc *FloatIncrementer) SetValue(v float64) {
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
func (inc *FloatIncrementer) CreateRenderer() fyne.WidgetRenderer {
	inc.upBtn.Importance = widget.LowImportance
	inc.downBtn.Importance = widget.LowImportance
	buttons := container.NewGridWithRows(2,
		borderedIncrementButton(inc.upBtn),
		borderedIncrementButton(inc.downBtn),
	)
	buttonsHalf := container.New(&halfHeightLayout{}, buttons)
	content := container.NewBorder(nil, nil, nil, buttonsHalf, inc.label)
	return widget.NewSimpleRenderer(content)
}

// Disable disables both increment/decrement controls.
func (inc *FloatIncrementer) Disable() {
	inc.upBtn.Disable()
	inc.downBtn.Disable()
	inc.Refresh()
}

// Enable enables both increment/decrement controls.
func (inc *FloatIncrementer) Enable() {
	inc.upBtn.Enable()
	inc.downBtn.Enable()
	inc.Refresh()
}
