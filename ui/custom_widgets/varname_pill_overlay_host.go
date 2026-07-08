package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

type varNamePillOverlayHost struct {
	widget.BaseWidget
	visual fyne.CanvasObject
	entry  *VarNameEntry
}

func newVarNamePillOverlayHost(visual fyne.CanvasObject, entry *VarNameEntry) *varNamePillOverlayHost {
	h := &varNamePillOverlayHost{visual: visual, entry: entry}
	h.ExtendBaseWidget(h)
	return h
}

func (h *varNamePillOverlayHost) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(h.visual)
}

func (h *varNamePillOverlayHost) Tapped(*fyne.PointEvent) {
	if h.entry == nil || h.entry.Disabled() {
		return
	}
	h.entry.focusOnCanvas()
}

func (h *varNamePillOverlayHost) TappedSecondary(ev *fyne.PointEvent) {
	if h.entry == nil {
		return
	}
	h.entry.TappedSecondary(ev)
}
