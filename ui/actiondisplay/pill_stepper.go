package actiondisplay

import (
	"fmt"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const pillStepperButtonGap = 2

// PillIntStepper is a caption-sized integer stepper for editable action pills.
type PillIntStepper struct {
	fynewidget.BaseWidget

	Value int
	Step  int
	Min   *int
	Max   *int

	OnChanged func(int)

	prefix     string
	actionType string
	prefixText *canvas.Text
	valueText  *canvas.Text
	valueHover *pillTipHover
	upBtn      *pillTipButton
	downBtn    *pillTipButton
	disabled   bool

	tips pillStepperTooltipState
}

// BindTooltipSink connects hover tips to the action-tooltip panel layer.
func (s *PillIntStepper) BindTooltipSink(sink TooltipSink) {
	s.tips.bindSink(sink)
}

func (s *PillIntStepper) scheduleTooltip(text string, absPos fyne.Position) {
	s.tips.scheduleTooltip(text, absPos)
}

func (s *PillIntStepper) cancelTooltip() {
	s.tips.cancelTooltip()
}

// NewPillIntStepper creates a compact int stepper sized for action tooltip pills.
func NewPillIntStepper(prefix string, value, step int, min, max *int, actionType string) *PillIntStepper {
	if step <= 0 {
		step = 1
	}
	s := &PillIntStepper{
		Value:      value,
		Step:       step,
		Min:        min,
		Max:        max,
		prefix:     prefix,
		actionType: actionType,
	}
	s.upBtn = newPillTipButton(theme.MenuDropUpIcon(), stepTooltipInt(step), s, func() { s.adjust(1) })
	s.downBtn = newPillTipButton(theme.MenuDropDownIcon(), stepTooltipInt(step), s, func() { s.adjust(-1) })
	s.upBtn.Importance = fynewidget.LowImportance
	s.downBtn.Importance = fynewidget.LowImportance
	s.prefixText = NewPillText(prefix + ": ")
	s.valueText = NewPillText(fmt.Sprintf("%d", value))
	s.valueHover = newPillTipHover(s)
	s.syncTooltips()
	s.ExtendBaseWidget(s)
	return s
}

func (s *PillIntStepper) syncTooltips() {
	stepTip := stepTooltipInt(s.Step)
	s.upBtn.SetToolTip(stepTip)
	s.downBtn.SetToolTip(stepTip)
	s.valueHover.SetToolTip(pillBoundsTooltipInt(s.Min, s.Max))
}

func (s *PillIntStepper) Disabled() bool {
	return s.disabled
}

func (s *PillIntStepper) syncEnabledVisual() {
	enabled := !s.disabled
	setPillTextEnabled(s.prefixText, enabled)
	setPillTextEnabled(s.valueText, enabled)
}

func (s *PillIntStepper) Enable() {
	s.disabled = false
	s.upBtn.Enable()
	s.downBtn.Enable()
	s.syncEnabledVisual()
}

func (s *PillIntStepper) Disable() {
	s.disabled = true
	s.upBtn.Disable()
	s.downBtn.Disable()
	s.syncEnabledVisual()
}

func (s *PillIntStepper) adjust(delta int) {
	if s.disabled {
		return
	}
	newVal := s.Value + delta*s.Step
	if s.Min != nil && newVal < *s.Min {
		newVal = *s.Min
	}
	if s.Max != nil && newVal > *s.Max {
		newVal = *s.Max
	}
	if newVal == s.Value {
		return
	}
	s.Value = newVal
	s.valueText.Text = fmt.Sprintf("%d", s.Value)
	s.valueText.Refresh()
	if s.OnChanged != nil {
		s.OnChanged(s.Value)
	}
	s.Refresh()
}

func (s *PillIntStepper) valueWidth() float32 {
	return fyne.MeasureText(s.valueText.Text, PillTextSize(), fyne.TextStyle{}).Width
}

func (s *PillIntStepper) MinSize() fyne.Size {
	btn := pillStepperButtonSize()
	w := s.prefixText.MinSize().Width + s.valueWidth() + btn.Width*2 + pillStepperButtonGap + 2
	return fyne.NewSize(w, PillLineHeight())
}

func (s *PillIntStepper) CreateRenderer() fyne.WidgetRenderer {
	return &pillIntStepperRenderer{stepper: s}
}

type pillIntStepperRenderer struct {
	stepper *PillIntStepper
}

func (r *pillIntStepperRenderer) Layout(size fyne.Size) {
	layoutPillStepperRow(size, r.stepper.prefixText, r.stepper.valueText, r.stepper.valueHover, r.stepper.upBtn, r.stepper.downBtn)
}

func (r *pillIntStepperRenderer) MinSize() fyne.Size {
	return r.stepper.MinSize()
}

func (r *pillIntStepperRenderer) Refresh() {
	r.stepper.syncEnabledVisual()
	r.stepper.prefixText.Refresh()
	r.stepper.valueText.Refresh()
	r.stepper.valueHover.Refresh()
	r.stepper.upBtn.Refresh()
	r.stepper.downBtn.Refresh()
}

func (r *pillIntStepperRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.stepper.prefixText, r.stepper.valueText, r.stepper.valueHover, r.stepper.upBtn, r.stepper.downBtn}
}

func (r *pillIntStepperRenderer) Destroy() {}

// PillFloatStepper is a caption-sized float stepper for editable action pills.
type PillFloatStepper struct {
	fynewidget.BaseWidget

	Value     float64
	Step      float64
	Min       *float64
	Max       *float64
	Precision int

	OnChanged func(float64)

	prefix     string
	actionType string
	prefixText *canvas.Text
	valueText  *canvas.Text
	valueHover *pillTipHover
	upBtn      *pillTipButton
	downBtn    *pillTipButton
	disabled   bool

	tips pillStepperTooltipState
}

// BindTooltipSink connects hover tips to the action-tooltip panel layer.
func (s *PillFloatStepper) BindTooltipSink(sink TooltipSink) {
	s.tips.bindSink(sink)
}

func (s *PillFloatStepper) scheduleTooltip(text string, absPos fyne.Position) {
	s.tips.scheduleTooltip(text, absPos)
}

func (s *PillFloatStepper) cancelTooltip() {
	s.tips.cancelTooltip()
}

// NewPillFloatStepper creates a compact float stepper sized for action tooltip pills.
func NewPillFloatStepper(prefix string, value, step float64, min, max *float64, precision int, actionType string) *PillFloatStepper {
	if step <= 0 {
		step = 1
	}
	if precision < 0 {
		precision = 2
	}
	s := &PillFloatStepper{
		Value:      value,
		Step:       step,
		Min:        min,
		Max:        max,
		Precision:  precision,
		prefix:     prefix,
		actionType: actionType,
	}
	stepTip := stepTooltipFloat(step, precision)
	s.upBtn = newPillTipButton(theme.MenuDropUpIcon(), stepTip, s, func() { s.adjust(1) })
	s.downBtn = newPillTipButton(theme.MenuDropDownIcon(), stepTip, s, func() { s.adjust(-1) })
	s.upBtn.Importance = fynewidget.LowImportance
	s.downBtn.Importance = fynewidget.LowImportance
	s.prefixText = NewPillText(prefix + ": ")
	s.valueText = NewPillText(fmt.Sprintf("%.*f", precision, value))
	s.valueHover = newPillTipHover(s)
	s.syncTooltips()
	s.ExtendBaseWidget(s)
	return s
}

func (s *PillFloatStepper) syncTooltips() {
	stepTip := stepTooltipFloat(s.Step, s.Precision)
	s.upBtn.SetToolTip(stepTip)
	s.downBtn.SetToolTip(stepTip)
	s.valueHover.SetToolTip(pillBoundsTooltipFloat(s.Min, s.Max, s.Precision))
}

func (s *PillFloatStepper) Disabled() bool {
	return s.disabled
}

func (s *PillFloatStepper) syncEnabledVisual() {
	enabled := !s.disabled
	setPillTextEnabled(s.prefixText, enabled)
	setPillTextEnabled(s.valueText, enabled)
}

func (s *PillFloatStepper) Enable() {
	s.disabled = false
	s.upBtn.Enable()
	s.downBtn.Enable()
	s.syncEnabledVisual()
}

func (s *PillFloatStepper) Disable() {
	s.disabled = true
	s.upBtn.Disable()
	s.downBtn.Disable()
	s.syncEnabledVisual()
}

func (s *PillFloatStepper) adjust(delta int) {
	if s.disabled {
		return
	}
	newVal := s.Value + float64(delta)*s.Step
	if s.Min != nil && newVal < *s.Min {
		newVal = *s.Min
	}
	if s.Max != nil && newVal > *s.Max {
		newVal = *s.Max
	}
	if newVal == s.Value {
		return
	}
	s.Value = newVal
	s.valueText.Text = fmt.Sprintf("%.*f", s.Precision, s.Value)
	s.valueText.Refresh()
	if s.OnChanged != nil {
		s.OnChanged(s.Value)
	}
	s.Refresh()
}

func (s *PillFloatStepper) valueWidth() float32 {
	return fyne.MeasureText(s.valueText.Text, PillTextSize(), fyne.TextStyle{}).Width
}

func (s *PillFloatStepper) MinSize() fyne.Size {
	btn := pillStepperButtonSize()
	w := s.prefixText.MinSize().Width + s.valueWidth() + btn.Width*2 + pillStepperButtonGap + 2
	return fyne.NewSize(w, PillLineHeight())
}

func (s *PillFloatStepper) CreateRenderer() fyne.WidgetRenderer {
	return &pillFloatStepperRenderer{stepper: s}
}

type pillFloatStepperRenderer struct {
	stepper *PillFloatStepper
}

func (r *pillFloatStepperRenderer) Layout(size fyne.Size) {
	layoutPillStepperRow(size, r.stepper.prefixText, r.stepper.valueText, r.stepper.valueHover, r.stepper.upBtn, r.stepper.downBtn)
}

func (r *pillFloatStepperRenderer) MinSize() fyne.Size {
	return r.stepper.MinSize()
}

func (r *pillFloatStepperRenderer) Refresh() {
	r.stepper.syncEnabledVisual()
	r.stepper.prefixText.Refresh()
	r.stepper.valueText.Refresh()
	r.stepper.valueHover.Refresh()
	r.stepper.upBtn.Refresh()
	r.stepper.downBtn.Refresh()
}

func (r *pillFloatStepperRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.stepper.prefixText, r.stepper.valueText, r.stepper.valueHover, r.stepper.upBtn, r.stepper.downBtn}
}

func (r *pillFloatStepperRenderer) Destroy() {}

func pillStepperButtonSize() fyne.Size {
	h := PillLineHeight()
	return fyne.NewSize(h, h)
}

func stepTooltipInt(step int) string {
	if step <= 0 {
		step = 1
	}
	return fmt.Sprintf("%d", step)
}

func stepTooltipFloat(step float64, precision int) string {
	if step <= 0 {
		step = 1
	}
	return fmt.Sprintf("%.*f", precision, step)
}

func pillBoundsTooltipInt(min, max *int) string {
	var parts []string
	if min != nil {
		parts = append(parts, fmt.Sprintf("Min: %d", *min))
	}
	if max != nil {
		parts = append(parts, fmt.Sprintf("Max: %d", *max))
	}
	return strings.Join(parts, ", ")
}

func pillBoundsTooltipFloat(min, max *float64, precision int) string {
	var parts []string
	if min != nil {
		parts = append(parts, fmt.Sprintf("Min: %.*f", precision, *min))
	}
	if max != nil {
		parts = append(parts, fmt.Sprintf("Max: %.*f", precision, *max))
	}
	return strings.Join(parts, ", ")
}

func layoutPillStepperRow(size fyne.Size, prefix, value fyne.CanvasObject, valueHover fyne.CanvasObject, upBtn, downBtn *pillTipButton) {
	rowH := PillLineHeight()
	yOff := (size.Height - rowH) / 2
	btnSize := pillStepperButtonSize()

	downX := size.Width - btnSize.Width
	upX := downX - pillStepperButtonGap - btnSize.Width

	prefixW := prefix.MinSize().Width
	valueW := upX - prefixW - 1
	if valueW < 0 {
		valueW = 0
	}

	prefix.Resize(fyne.NewSize(prefixW, rowH))
	prefix.Move(fyne.NewPos(0, yOff))
	value.Resize(fyne.NewSize(valueW, rowH))
	value.Move(fyne.NewPos(prefixW, yOff))
	if valueHover != nil {
		valueHover.Resize(fyne.NewSize(valueW, rowH))
		valueHover.Move(fyne.NewPos(prefixW, yOff))
	}

	btnY := yOff + (rowH-btnSize.Height)/2
	upBtn.Resize(btnSize)
	upBtn.Move(fyne.NewPos(upX, btnY))
	downBtn.Resize(btnSize)
	downBtn.Move(fyne.NewPos(downX, btnY))
}
