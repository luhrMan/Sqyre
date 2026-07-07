package actiondisplay

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/widget"
)

// NewPillIconButton returns a compact icon control sized for pill toolbars.
func NewPillIconButton(res fyne.Resource, onTapped func()) *pillIconButton {
	b := &pillIconButton{resource: res, onTapped: onTapped}
	b.icon = canvas.NewImageFromResource(res)
	b.icon.FillMode = canvas.ImageFillContain
	b.ExtendBaseWidget(b)
	return b
}

type pillIconButton struct {
	widget.BaseWidget

	resource fyne.Resource
	onTapped func()
	icon     *canvas.Image
}

func (b *pillIconButton) MinSize() fyne.Size {
	return fyne.NewSquareSize(PillLineHeight())
}

func (b *pillIconButton) Tapped(*fyne.PointEvent) {
	if b.onTapped != nil {
		b.onTapped()
	}
}

func (b *pillIconButton) CreateRenderer() fyne.WidgetRenderer {
	return &pillIconButtonRenderer{button: b}
}

type pillIconButtonRenderer struct {
	button *pillIconButton
}

func (r *pillIconButtonRenderer) Layout(size fyne.Size) {
	r.button.icon.Resize(size)
	r.button.icon.Move(fyne.NewPos(0, 0))
}

func (r *pillIconButtonRenderer) MinSize() fyne.Size {
	return r.button.MinSize()
}

func (r *pillIconButtonRenderer) Refresh() {
	r.button.icon.Refresh()
}

func (r *pillIconButtonRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.button.icon}
}

func (r *pillIconButtonRenderer) Destroy() {}

var _ fyne.Tappable = (*pillIconButton)(nil)
