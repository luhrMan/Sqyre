package custom_widgets

import (
	"image"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
)

// ZoomableImageView displays an image with wheel zoom and drag pan.
type ZoomableImageView struct {
	widget.BaseWidget

	source   image.Image
	transform ImageViewTransform
	minSize  fyne.Size

	img *canvas.Image

	panning  bool
	panStart fyne.Position
	panBaseX float32
	panBaseY float32
}

// NewZoomableImageView creates an empty zoomable preview.
func NewZoomableImageView() *ZoomableImageView {
	z := &ZoomableImageView{
		img:     canvas.NewImageFromImage(nil),
		minSize: fyne.NewSize(320, 240),
	}
	z.img.FillMode = canvas.ImageFillStretch
	z.ExtendBaseWidget(z)
	return z
}

// SetMinSize sets the minimum viewport size for layout.
func (z *ZoomableImageView) SetMinSize(size fyne.Size) {
	z.minSize = size
	z.Refresh()
}

// SetImage replaces the displayed image and resets zoom/pan.
func (z *ZoomableImageView) SetImage(img image.Image) {
	z.source = img
	z.transform = ResetImageViewTransform()
	z.img.Image = img
	z.layoutImage()
	z.Refresh()
}

// ResetView fits the current image back into the viewport.
func (z *ZoomableImageView) ResetView() {
	z.transform = ResetImageViewTransform()
	z.layoutImage()
	z.Refresh()
}

func (z *ZoomableImageView) imageSize() fyne.Size {
	if z.source == nil {
		return fyne.Size{}
	}
	b := z.source.Bounds()
	return ImagePixelSize(b.Dx(), b.Dy())
}

func (z *ZoomableImageView) layoutImage() {
	size := z.Size()
	if size.Width <= 0 || size.Height <= 0 {
		return
	}
	imgSize := z.imageSize()
	if imgSize.Width <= 0 || imgSize.Height <= 0 {
		z.img.Hide()
		return
	}
	z.img.Show()
	x, y, w, h := ImageContentRect(size, imgSize, z.transform)
	z.img.Resize(fyne.NewSize(w, h))
	z.img.Move(fyne.NewPos(x, y))
	z.img.Refresh()
}

func (z *ZoomableImageView) CreateRenderer() fyne.WidgetRenderer {
	root := container.NewWithoutLayout(z.img)
	return &zoomableImageRenderer{view: z, root: root, objects: []fyne.CanvasObject{root}}
}

func (z *ZoomableImageView) MinSize() fyne.Size {
	return z.minSize
}

func (z *ZoomableImageView) Resize(size fyne.Size) {
	z.BaseWidget.Resize(size)
	z.layoutImage()
}

func (z *ZoomableImageView) Scrolled(ev *fyne.ScrollEvent) {
	if z.source == nil || ev == nil {
		return
	}
	factor := ScrollZoomFactor(ev.Scrolled.DY)
	if factor == 1 {
		return
	}
	z.transform = ZoomImageAtCursor(z.Size(), z.imageSize(), z.transform, ev.Position, factor)
	z.layoutImage()
	z.Refresh()
}

func (z *ZoomableImageView) MouseDown(ev *desktop.MouseEvent) {
	if z.source == nil || ev.Button != desktop.MouseButtonPrimary {
		return
	}
	z.panning = true
	z.panStart = ev.Position
	z.panBaseX = z.transform.PanX
	z.panBaseY = z.transform.PanY
}

func (z *ZoomableImageView) MouseUp(*desktop.MouseEvent) {
	z.panning = false
}

func (z *ZoomableImageView) MouseIn(*desktop.MouseEvent)  {}
func (z *ZoomableImageView) MouseOut()                      {}
func (z *ZoomableImageView) MouseMoved(ev *desktop.MouseEvent) {
	if !z.panning || z.source == nil {
		return
	}
	z.transform.PanX = z.panBaseX + (ev.Position.X - z.panStart.X)
	z.transform.PanY = z.panBaseY + (ev.Position.Y - z.panStart.Y)
	z.transform = ClampImagePan(z.Size(), z.imageSize(), z.transform)
	z.layoutImage()
	z.Refresh()
}

type zoomableImageRenderer struct {
	view    *ZoomableImageView
	root    *fyne.Container
	objects []fyne.CanvasObject
}

func (r *zoomableImageRenderer) Layout(size fyne.Size) {
	r.root.Resize(size)
	r.view.layoutImage()
}

func (r *zoomableImageRenderer) MinSize() fyne.Size {
	return r.view.minSize
}

func (r *zoomableImageRenderer) Refresh() {
	r.view.layoutImage()
	canvas.Refresh(r.root)
}

func (r *zoomableImageRenderer) Objects() []fyne.CanvasObject { return r.objects }
func (r *zoomableImageRenderer) Destroy()                     {}

var (
	_ fyne.Scrollable   = (*ZoomableImageView)(nil)
	_ desktop.Mouseable = (*ZoomableImageView)(nil)
	_ desktop.Hoverable = (*ZoomableImageView)(nil)
)
