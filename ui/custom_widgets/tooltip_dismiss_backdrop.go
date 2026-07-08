package custom_widgets

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	fynewidget "fyne.io/fyne/v2/widget"
)

// TooltipDismissBackdrop is a transparent full-area tap target that dismisses a pinned tooltip.
type TooltipDismissBackdrop struct {
	fynewidget.BaseWidget

	onDismiss func()
}

var _ fyne.Tappable = (*TooltipDismissBackdrop)(nil)

// NewTooltipDismissBackdrop creates a backdrop that calls onDismiss on primary tap.
func NewTooltipDismissBackdrop(onDismiss func()) *TooltipDismissBackdrop {
	b := &TooltipDismissBackdrop{onDismiss: onDismiss}
	b.ExtendBaseWidget(b)
	return b
}

func (b *TooltipDismissBackdrop) Tapped(*fyne.PointEvent) {
	if b.onDismiss == nil {
		return
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(b)
	if c != nil && c.Overlays().Top() != nil {
		return
	}
	b.onDismiss()
}

func (b *TooltipDismissBackdrop) CreateRenderer() fyne.WidgetRenderer {
	return &tooltipDismissBackdropRenderer{hit: canvas.NewRectangle(color.Transparent)}
}

type tooltipDismissBackdropRenderer struct {
	hit *canvas.Rectangle
}

func (r *tooltipDismissBackdropRenderer) Layout(size fyne.Size) {
	r.hit.Resize(size)
}

func (r *tooltipDismissBackdropRenderer) MinSize() fyne.Size {
	return fyne.NewSize(0, 0)
}

func (r *tooltipDismissBackdropRenderer) Refresh() {}

func (r *tooltipDismissBackdropRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.hit}
}

func (r *tooltipDismissBackdropRenderer) Destroy() {}
