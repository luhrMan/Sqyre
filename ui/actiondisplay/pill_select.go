package actiondisplay

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

// PillSelect is a caption-sized option cycler for editable action pills.
type PillSelect struct {
	fynewidget.BaseWidget

	Label   string
	Options []string
	Value   string
	Format  func(string) string

	OnChanged func(string)

	prefixText *canvas.Text
	valueText  *canvas.Text
	prevBtn    *pillTipButton
	nextBtn    *pillTipButton
	disabled   bool
}

// NewPillSelect creates a compact select sized for action tooltip pills.
// format renders option values for display; pass nil to show raw values.
func NewPillSelect(label string, options []string, value string, format func(string) string) *PillSelect {
	if len(options) == 0 {
		options = []string{""}
	}
	value = normalizePillSelectValue(value, options, options[0])
	s := &PillSelect{
		Label:   label,
		Options: append([]string(nil), options...),
		Value:   value,
		Format:  format,
	}
	s.prevBtn = newPillTipButton(theme.NavigateBackIcon(), "Previous option", s, func() { s.cycle(-1) })
	s.nextBtn = newPillTipButton(theme.NavigateNextIcon(), "Next option", s, func() { s.cycle(1) })
	s.prevBtn.Importance = fynewidget.LowImportance
	s.nextBtn.Importance = fynewidget.LowImportance
	s.prefixText = NewPillText(label + ": ")
	s.valueText = NewPillText(s.displayValue(value))
	s.ExtendBaseWidget(s)
	return s
}

func normalizePillSelectValue(value string, options []string, fallback string) string {
	for _, opt := range options {
		if opt == value {
			return value
		}
	}
	return fallback
}

func (s *PillSelect) displayValue(value string) string {
	if s.Format != nil {
		return s.Format(value)
	}
	return value
}

func (s *PillSelect) Disabled() bool {
	return s.disabled
}

func (s *PillSelect) syncEnabledVisual() {
	enabled := !s.disabled
	setPillTextEnabled(s.prefixText, enabled)
	setPillTextEnabled(s.valueText, enabled)
}

func (s *PillSelect) Enable() {
	s.disabled = false
	s.prevBtn.Enable()
	s.nextBtn.Enable()
	s.syncEnabledVisual()
}

func (s *PillSelect) Disable() {
	s.disabled = true
	s.prevBtn.Disable()
	s.nextBtn.Disable()
	s.syncEnabledVisual()
}

func (s *PillSelect) selectedIndex() int {
	for i, opt := range s.Options {
		if opt == s.Value {
			return i
		}
	}
	return 0
}

func (s *PillSelect) scheduleTooltip(string, fyne.Position) {}

func (s *PillSelect) cancelTooltip() {}

func (s *PillSelect) cycle(delta int) {
	if s.disabled || len(s.Options) == 0 {
		return
	}
	idx := s.selectedIndex() + delta
	n := len(s.Options)
	idx = ((idx % n) + n) % n
	next := s.Options[idx]
	if next == s.Value {
		return
	}
	s.Value = next
	s.valueText.Text = s.displayValue(next)
	s.valueText.Refresh()
	if s.OnChanged != nil {
		s.OnChanged(next)
	}
	s.Refresh()
}

func (s *PillSelect) valueWidth() float32 {
	return fyne.MeasureText(s.valueText.Text, PillTextSize(), fyne.TextStyle{}).Width
}

func (s *PillSelect) MinSize() fyne.Size {
	btn := pillStepperButtonSize()
	w := s.prefixText.MinSize().Width + s.valueWidth() + btn.Width*2 + pillStepperButtonGap + 2
	return fyne.NewSize(w, PillLineHeight())
}

func (s *PillSelect) CreateRenderer() fyne.WidgetRenderer {
	return &pillSelectRenderer{pill: s}
}

type pillSelectRenderer struct {
	pill *PillSelect
}

func (r *pillSelectRenderer) Layout(size fyne.Size) {
	layoutPillStepperRow(size, r.pill.prefixText, r.pill.valueText, nil, r.pill.prevBtn, r.pill.nextBtn)
}

func (r *pillSelectRenderer) MinSize() fyne.Size {
	return r.pill.MinSize()
}

func (r *pillSelectRenderer) Refresh() {
	r.pill.syncEnabledVisual()
	r.pill.prefixText.Refresh()
	display := r.pill.displayValue(r.pill.Value)
	if r.pill.valueText.Text != display {
		r.pill.valueText.Text = display
	}
	r.pill.valueText.Refresh()
	r.pill.prevBtn.Refresh()
	r.pill.nextBtn.Refresh()
}

func (r *pillSelectRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.pill.prefixText, r.pill.valueText, r.pill.nextBtn, r.pill.prevBtn}
}

func (r *pillSelectRenderer) Destroy() {}

// WrapPillSelect wraps a PillSelect in pill chrome.
func WrapPillSelect(sel *PillSelect, actionType string) fyne.CanvasObject {
	return PillChrome(sel, actionType)
}

// NewDisplaySelectPill shows a label with the formatted selected option.
func NewDisplaySelectPill(label, value string, format func(string) string, actionType string) fyne.CanvasObject {
	display := value
	if format != nil {
		display = format(value)
	}
	return NewDisplayPill(fmt.Sprintf("%s: %s", label, display), actionType)
}
