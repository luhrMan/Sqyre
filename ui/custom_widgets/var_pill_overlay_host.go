package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// pillOverlayHost forwards taps to the entry so the field can be focused while pills are shown.
type pillOverlayHost struct {
	widget.BaseWidget
	visual fyne.CanvasObject
	entry  *VarEntry
}

func newPillOverlayHost(visual fyne.CanvasObject, entry *VarEntry) *pillOverlayHost {
	h := &pillOverlayHost{visual: visual, entry: entry}
	h.ExtendBaseWidget(h)
	return h
}

func (h *pillOverlayHost) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(h.visual)
}

func (h *pillOverlayHost) Tapped(*fyne.PointEvent) {
	if h.entry == nil || h.entry.Disabled() {
		return
	}
	h.entry.focusOnCanvas()
}

func (h *pillOverlayHost) TappedSecondary(ev *fyne.PointEvent) {
	if h.entry == nil {
		return
	}
	h.entry.TappedSecondary(ev)
}

func (o *variableRefOverlay) object(entry *VarEntry) fyne.CanvasObject {
	if o.host == nil {
		o.host = newPillOverlayHost(o.root, entry)
	}
	return o.host
}
