package macro

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/driver/desktop"
	fynewidget "fyne.io/fyne/v2/widget"
)

// actionIconTooltipHover is an invisible overlay on the tree row action icon.
// It shows the same rich action tooltip as hovering the action display.
type actionIconTooltipHover struct {
	fynewidget.BaseWidget

	target *actionDisplayTooltipHover
}

var (
	_ desktop.Hoverable      = (*actionIconTooltipHover)(nil)
	_ fyne.SecondaryTappable = (*actionIconTooltipHover)(nil)
)

func newActionIconTooltipHover() *actionIconTooltipHover {
	h := &actionIconTooltipHover{}
	h.ExtendBaseWidget(h)
	return h
}

func (h *actionIconTooltipHover) bindActionTooltip(target *actionDisplayTooltipHover) {
	h.target = target
}

func (h *actionIconTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.iconMouseIn(e)
	}
}

func (h *actionIconTooltipHover) MouseOut() {
	if h.target != nil {
		h.target.iconMouseOut()
	}
}

func (h *actionIconTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.iconMouseMoved(e)
	}
}

func (h *actionIconTooltipHover) TappedSecondary(*fyne.PointEvent) {
	if h.target != nil {
		h.target.openTooltipEdit()
	}
}

// actionRowTooltipHover is an invisible overlay on the macro tree action body.
// It deliberately does not cover the row remove button so delete taps stay reachable.
type actionRowTooltipHover struct {
	fynewidget.BaseWidget

	target *actionDisplayTooltipHover
}

var (
	_ desktop.Hoverable      = (*actionRowTooltipHover)(nil)
	_ fyne.Tappable          = (*actionRowTooltipHover)(nil)
	_ fyne.SecondaryTappable = (*actionRowTooltipHover)(nil)
)

func newActionRowTooltipHover() *actionRowTooltipHover {
	h := &actionRowTooltipHover{}
	h.ExtendBaseWidget(h)
	return h
}

func (h *actionRowTooltipHover) bindActionTooltip(target *actionDisplayTooltipHover) {
	h.target = target
}

func (h *actionRowTooltipHover) MouseIn(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.rowMouseIn(e)
	}
}

func (h *actionRowTooltipHover) MouseOut() {
	if h.target != nil {
		h.target.rowMouseOut()
	}
}

func (h *actionRowTooltipHover) MouseMoved(e *desktop.MouseEvent) {
	if h.target != nil {
		h.target.rowMouseMoved(e)
	}
}

// Tapped forwards primary taps to the action display. This overlay sits on top
// of the whole row, so without it Fyne routes clicks here (SecondaryTappable
// wins the hit test) and drops them, breaking row selection and double-click edit.
func (h *actionRowTooltipHover) Tapped(pe *fyne.PointEvent) {
	if h.target != nil {
		h.target.Tapped(pe)
	}
}

func (h *actionRowTooltipHover) TappedSecondary(*fyne.PointEvent) {
	if h.target != nil {
		h.target.openTooltipEdit()
	}
}

func (h *actionRowTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &actionRowTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type actionRowTooltipHoverRenderer struct {
	hover *actionRowTooltipHover
	hit   *canvas.Rectangle
}

func (r *actionRowTooltipHoverRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *actionRowTooltipHoverRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *actionRowTooltipHoverRenderer) Refresh() {}

func (r *actionRowTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *actionRowTooltipHoverRenderer) Destroy() {}

func (h *actionIconTooltipHover) CreateRenderer() fyne.WidgetRenderer {
	return &actionIconTooltipHoverRenderer{hover: h, hit: canvas.NewRectangle(color.Transparent)}
}

type actionIconTooltipHoverRenderer struct {
	hover *actionIconTooltipHover
	hit   *canvas.Rectangle
}

func (r *actionIconTooltipHoverRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *actionIconTooltipHoverRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *actionIconTooltipHoverRenderer) Refresh() {}

func (r *actionIconTooltipHoverRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *actionIconTooltipHoverRenderer) Destroy() {}
