package actiondisplay

import (
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	fynewidget "fyne.io/fyne/v2/widget"
)

// NewDisplayTogglePill shows a label with an on/off toggle (read-only).
func NewDisplayTogglePill(label string, on bool, actionType string) fyne.CanvasObject {
	toggle := custom_widgets.NewCompactToggle(nil)
	toggle.SetToggled(on)
	toggle.Disable()
	return PillChrome(newPillToggleRow(NewPillText(label), toggle), actionType)
}

// PillToggle is a caption-height pill with a label and compact toggle switch.
type PillToggle struct {
	fynewidget.BaseWidget

	Label     string
	Value     bool
	OnChanged func(bool)
	disabled  bool

	labelText *canvas.Text
	toggle    *custom_widgets.Toggle
}

// NewPillToggle creates a compact bool pill for tooltip edit fields.
func NewPillToggle(label string, value bool) *PillToggle {
	p := &PillToggle{Label: label, Value: value}
	p.labelText = NewPillText(label)
	p.toggle = custom_widgets.NewCompactToggle(func(b bool) {
		p.Value = b
		if p.OnChanged != nil {
			p.OnChanged(b)
		}
	})
	p.toggle.SetToggled(value)
	p.ExtendBaseWidget(p)
	return p
}

func (p *PillToggle) Disabled() bool {
	return p.disabled
}

func (p *PillToggle) Enable() {
	p.disabled = false
	p.toggle.Enable()
	setPillTextEnabled(p.labelText, true)
}

func (p *PillToggle) Disable() {
	p.disabled = true
	p.toggle.Disable()
	setPillTextEnabled(p.labelText, false)
}

// SetLabel updates the toggle caption and relayouts the pill.
func (p *PillToggle) SetLabel(label string) {
	p.Label = label
	p.labelText.Text = label
	p.labelText.Refresh()
	p.Refresh()
}

func (p *PillToggle) MinSize() fyne.Size {
	return pillToggleMinSize(p.labelText, p.toggle)
}

func (p *PillToggle) CreateRenderer() fyne.WidgetRenderer {
	return &pillToggleRenderer{label: p.labelText, toggle: p.toggle}
}

// WrapPillToggle wraps a PillToggle in pill chrome.
func WrapPillToggle(pill *PillToggle, actionType string) fyne.CanvasObject {
	return PillChrome(pill, actionType)
}

func newPillToggleRow(label *canvas.Text, toggle *custom_widgets.Toggle) *pillToggleRow {
	r := &pillToggleRow{label: label, toggle: toggle}
	r.ExtendBaseWidget(r)
	return r
}

type pillToggleRow struct {
	fynewidget.BaseWidget
	label  *canvas.Text
	toggle *custom_widgets.Toggle
}

func (r *pillToggleRow) MinSize() fyne.Size {
	return pillToggleMinSize(r.label, r.toggle)
}

func (r *pillToggleRow) CreateRenderer() fyne.WidgetRenderer {
	return &pillToggleRenderer{label: r.label, toggle: r.toggle}
}

type pillToggleRenderer struct {
	label  *canvas.Text
	toggle *custom_widgets.Toggle
}

func (r *pillToggleRenderer) Layout(size fyne.Size) {
	layoutPillToggleRow(size, r.label, r.toggle)
}

func (r *pillToggleRenderer) MinSize() fyne.Size {
	return pillToggleMinSize(r.label, r.toggle)
}

func (r *pillToggleRenderer) Refresh() {
	r.label.Refresh()
	r.toggle.Refresh()
}

func (r *pillToggleRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.label, r.toggle}
}

func (r *pillToggleRenderer) Destroy() {}

func pillToggleMinSize(label *canvas.Text, toggle *custom_widgets.Toggle) fyne.Size {
	labelW := label.MinSize().Width
	toggleSize := toggle.MinSize()
	h := PillLineHeight()
	if toggleSize.Height > h {
		h = toggleSize.Height
	}
	return fyne.NewSize(labelW+toggleSize.Width, h)
}

func layoutPillToggleRow(size fyne.Size, label *canvas.Text, toggle *custom_widgets.Toggle) {
	rowH := PillLineHeight()
	toggleSize := toggle.MinSize()
	contentH := rowH
	if toggleSize.Height > contentH {
		contentH = toggleSize.Height
	}
	yOff := (size.Height - contentH) / 2

	labelW := label.MinSize().Width
	label.Resize(fyne.NewSize(labelW, rowH))
	label.Move(fyne.NewPos(0, yOff+(contentH-rowH)/2))

	toggleY := yOff + (contentH-toggleSize.Height)/2
	toggle.Resize(toggleSize)
	toggle.Move(fyne.NewPos(labelW, toggleY))
}
