package actiondisplay

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

// PillDropdown is a caption-sized dropdown for editable action pills.
type PillDropdown struct {
	fynewidget.BaseWidget

	Label   string
	Options []string
	Value   string
	Format  func(string) string

	OnChanged func(string)

	prefixText *canvas.Text
	valueText  *canvas.Text
	menuBtn    *pillTipButton
	disabled   bool
}

// NewPillDropdown creates a compact dropdown sized for action tooltip pills.
// format renders option values for display; pass nil to show raw values.
func NewPillDropdown(label string, options []string, value string, format func(string) string) *PillDropdown {
	if len(options) == 0 {
		options = []string{""}
	}
	value = normalizePillSelectValue(value, options, options[0])
	d := &PillDropdown{
		Label:   label,
		Options: append([]string(nil), options...),
		Value:   value,
		Format:  format,
	}
	d.menuBtn = newPillTipButton(theme.MenuDropDownIcon(), "Choose option", d, d.showMenu)
	d.menuBtn.Importance = fynewidget.LowImportance
	d.prefixText = NewPillText(label + ": ")
	d.valueText = NewPillText(d.displayValue(value))
	d.ExtendBaseWidget(d)
	return d
}

func (d *PillDropdown) displayValue(value string) string {
	if d.Format != nil {
		return d.Format(value)
	}
	return value
}

func (d *PillDropdown) Disabled() bool {
	return d.disabled
}

func (d *PillDropdown) syncEnabledVisual() {
	enabled := !d.disabled
	setPillTextEnabled(d.prefixText, enabled)
	setPillTextEnabled(d.valueText, enabled)
}

func (d *PillDropdown) Enable() {
	d.disabled = false
	d.menuBtn.Enable()
	d.syncEnabledVisual()
}

func (d *PillDropdown) Disable() {
	d.disabled = true
	d.menuBtn.Disable()
	d.syncEnabledVisual()
}

func (d *PillDropdown) setValue(next string) {
	if d.disabled || next == d.Value {
		return
	}
	d.Value = next
	d.valueText.Text = d.displayValue(next)
	d.valueText.Refresh()
	if d.OnChanged != nil {
		d.OnChanged(next)
	}
	d.Refresh()
}

func (d *PillDropdown) showMenu() {
	if d.disabled || len(d.Options) == 0 {
		return
	}
	driver := fyne.CurrentApp().Driver()
	host := d
	pos := driver.AbsolutePositionForObject(host)
	c := driver.CanvasForObject(host)
	if c == nil {
		return
	}
	items := make([]*fyne.MenuItem, len(d.Options))
	for i, opt := range d.Options {
		opt := opt
		items[i] = fyne.NewMenuItem(d.displayValue(opt), func() {
			d.setValue(opt)
		})
	}
	menuPos := pos.Add(fyne.NewPos(0, d.Size().Height))
	fynewidget.ShowPopUpMenuAtPosition(fyne.NewMenu("", items...), c, menuPos)
}

// NewDisplayDropdownPill shows a label and selected option as read-only text (no menu button).
func NewDisplayDropdownPill(label, value string, format func(string) string, actionType string) fyne.CanvasObject {
	display := value
	if format != nil {
		display = format(value)
	}
	return PillChrome(NewPillInlineField(label+": ", NewPillText(display)), actionType)
}

func layoutPillDropdownRow(size fyne.Size, prefix, value fyne.CanvasObject, menuBtn *pillTipButton) {
	rowH := PillLineHeight()
	yOff := (size.Height - rowH) / 2
	btnSize := pillStepperButtonSize()
	btnX := size.Width - btnSize.Width

	prefixW := prefix.MinSize().Width
	valueW := btnX - prefixW - 1
	if valueW < 0 {
		valueW = 0
	}

	prefix.Resize(fyne.NewSize(prefixW, rowH))
	prefix.Move(fyne.NewPos(0, yOff))
	value.Resize(fyne.NewSize(valueW, rowH))
	value.Move(fyne.NewPos(prefixW, yOff))

	btnY := yOff + (rowH-btnSize.Height)/2
	menuBtn.Resize(btnSize)
	menuBtn.Move(fyne.NewPos(btnX, btnY))
}

func (d *PillDropdown) scheduleTooltip(string, fyne.Position) {}

func (d *PillDropdown) cancelTooltip() {}

func (d *PillDropdown) valueWidth() float32 {
	return fyne.MeasureText(d.valueText.Text, PillTextSize(), fyne.TextStyle{}).Width
}

func (d *PillDropdown) MinSize() fyne.Size {
	btn := pillStepperButtonSize()
	w := d.prefixText.MinSize().Width + d.valueWidth() + btn.Width + 2
	return fyne.NewSize(w, PillLineHeight())
}

func (d *PillDropdown) CreateRenderer() fyne.WidgetRenderer {
	return &pillDropdownRenderer{pill: d}
}

type pillDropdownRenderer struct {
	pill *PillDropdown
}

func (r *pillDropdownRenderer) Layout(size fyne.Size) {
	layoutPillDropdownRow(size, r.pill.prefixText, r.pill.valueText, r.pill.menuBtn)
}

func (r *pillDropdownRenderer) MinSize() fyne.Size {
	return r.pill.MinSize()
}

func (r *pillDropdownRenderer) Refresh() {
	r.pill.syncEnabledVisual()
	r.pill.prefixText.Refresh()
	display := r.pill.displayValue(r.pill.Value)
	if r.pill.valueText.Text != display {
		r.pill.valueText.Text = display
	}
	r.pill.valueText.Refresh()
	r.pill.menuBtn.Refresh()
}

func (r *pillDropdownRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.pill.prefixText, r.pill.valueText, r.pill.menuBtn}
}

func (r *pillDropdownRenderer) Destroy() {}

// WrapPillDropdown wraps a PillDropdown in pill chrome.
func WrapPillDropdown(sel *PillDropdown, actionType string) fyne.CanvasObject {
	return PillChrome(sel, actionType)
}
