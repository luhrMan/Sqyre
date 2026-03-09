package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// TappableIcon is a lightweight clickable icon (no button chrome).
// Useful in tight layouts where a full Button would be too large.
type TappableIcon struct {
	widget.BaseWidget

	Resource fyne.Resource
	OnTapped func()

	icon *widget.Icon
}

func NewTappableIcon(res fyne.Resource, onTapped func()) *TappableIcon {
	t := &TappableIcon{Resource: res, OnTapped: onTapped}
	t.ExtendBaseWidget(t)
	return t
}

func (t *TappableIcon) Tapped(*fyne.PointEvent) {
	if t.OnTapped != nil {
		t.OnTapped()
	}
}

func (t *TappableIcon) CreateRenderer() fyne.WidgetRenderer {
	t.icon = widget.NewIcon(t.Resource)
	return widget.NewSimpleRenderer(t.icon)
}

func (t *TappableIcon) Refresh() {
	if t.icon != nil {
		t.icon.SetResource(t.Resource)
	}
	t.BaseWidget.Refresh()
}
