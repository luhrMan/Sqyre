package custom_widgets

import (
	"image"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
)

// CollectionGridView shows a static collection image with a rows×cols overlay.
// When selectable is true, click/drag selects a 1-based inclusive cell rectangle.
// Wheel zooms; drag pans when zoomed above 1.
type CollectionGridView struct {
	widget.BaseWidget

	img        *canvas.Image
	overlay    *fyne.Container
	rows, cols int
	selectable bool

	transform ImageViewTransform

	// selection is 1-based inclusive; zero means none
	r1, c1, r2, c2 int
	dragging        bool
	dragR, dragC    int

	panning  bool
	panStart fyne.Position
	panBaseX float32
	panBaseY float32

	OnSelectionChanged func(r1, c1, r2, c2 int)
}

// NewCollectionGridView creates a non-selectable grid preview.
func NewCollectionGridView() *CollectionGridView {
	g := &CollectionGridView{
		img:       canvas.NewImageFromImage(nil),
		overlay:   container.NewWithoutLayout(),
		transform: ResetImageViewTransform(),
	}
	g.img.FillMode = canvas.ImageFillStretch
	g.ExtendBaseWidget(g)
	return g
}

// SetSelectable enables click/drag cell selection.
func (g *CollectionGridView) SetSelectable(v bool) {
	g.selectable = v
}

// SetImage sets the background image (may be nil).
func (g *CollectionGridView) SetImage(img image.Image) {
	g.img.Image = img
	g.transform = ResetImageViewTransform()
	g.layoutImage()
	g.rebuildOverlay()
}

// SetGrid sets the row/col counts (must be >= 1).
func (g *CollectionGridView) SetGrid(rows, cols int) {
	if rows < 1 {
		rows = 1
	}
	if cols < 1 {
		cols = 1
	}
	g.rows, g.cols = rows, cols
	g.rebuildOverlay()
}

// Selection returns the current 1-based inclusive selection, or ok=false if none.
func (g *CollectionGridView) Selection() (r1, c1, r2, c2 int, ok bool) {
	if g.r1 < 1 {
		return 0, 0, 0, 0, false
	}
	return g.r1, g.c1, g.r2, g.c2, true
}

// SetSelection sets the highlighted range (1-based inclusive). Pass zeros to clear.
func (g *CollectionGridView) SetSelection(r1, c1, r2, c2 int) {
	if r1 < 1 || c1 < 1 {
		g.r1, g.c1, g.r2, g.c2 = 0, 0, 0, 0
	} else {
		if r1 > r2 {
			r1, r2 = r2, r1
		}
		if c1 > c2 {
			c1, c2 = c2, c1
		}
		g.r1, g.c1, g.r2, g.c2 = r1, c1, r2, c2
	}
	g.rebuildOverlay()
}

// ClearSelection clears the highlight.
func (g *CollectionGridView) ClearSelection() {
	g.SetSelection(0, 0, 0, 0)
}

func (g *CollectionGridView) CreateRenderer() fyne.WidgetRenderer {
	root := container.NewWithoutLayout(g.img, g.overlay)
	return &collectionGridRenderer{grid: g, root: root, objects: []fyne.CanvasObject{root}}
}

func (g *CollectionGridView) MinSize() fyne.Size {
	return fyne.NewSize(320, 240)
}

func (g *CollectionGridView) Resize(size fyne.Size) {
	g.BaseWidget.Resize(size)
	g.layoutImage()
	g.rebuildOverlay()
}

func (g *CollectionGridView) imageSize() fyne.Size {
	if g.img.Image == nil {
		return fyne.Size{}
	}
	b := g.img.Image.Bounds()
	return ImagePixelSize(b.Dx(), b.Dy())
}

func (g *CollectionGridView) imageContentRect() (x, y, w, h float32) {
	return ImageContentRect(g.Size(), g.imageSize(), g.transform)
}

func (g *CollectionGridView) layoutImage() {
	size := g.Size()
	imgSize := g.imageSize()
	if size.Width <= 0 || size.Height <= 0 || imgSize.Width <= 0 || imgSize.Height <= 0 {
		g.img.Hide()
		return
	}
	g.img.Show()
	x, y, w, h := g.imageContentRect()
	g.img.Resize(fyne.NewSize(w, h))
	g.img.Move(fyne.NewPos(x, y))
	g.overlay.Resize(size)
	g.overlay.Move(fyne.NewPos(0, 0))
}

func (g *CollectionGridView) rebuildOverlay() {
	if g.overlay == nil {
		return
	}
	g.overlay.Objects = nil
	if g.rows < 1 || g.cols < 1 {
		g.overlay.Refresh()
		return
	}
	ox, oy, ow, oh := g.imageContentRect()
	if ow <= 0 || oh <= 0 {
		g.overlay.Refresh()
		return
	}

	// Selection highlight
	if g.r1 >= 1 {
		r1, c1, r2, c2 := g.r1, g.c1, g.r2, g.c2
		sx := ox + float32(c1-1)*ow/float32(g.cols)
		sy := oy + float32(r1-1)*oh/float32(g.rows)
		sw := float32(c2-c1+1) * ow / float32(g.cols)
		sh := float32(r2-r1+1) * oh / float32(g.rows)
		sel := canvas.NewRectangle(color.NRGBA{R: 80, G: 160, B: 255, A: 60})
		sel.StrokeColor = color.NRGBA{R: 40, G: 120, B: 220, A: 220}
		sel.StrokeWidth = 2
		sel.Resize(fyne.NewSize(sw, sh))
		sel.Move(fyne.NewPos(sx, sy))
		g.overlay.Objects = append(g.overlay.Objects, sel)
	}

	// Grid lines
	lineColor := color.NRGBA{R: 255, G: 255, B: 255, A: 180}
	for c := 0; c <= g.cols; c++ {
		x := ox + float32(c)*ow/float32(g.cols)
		ln := canvas.NewLine(lineColor)
		ln.StrokeWidth = 1
		ln.Position1 = fyne.NewPos(x, oy)
		ln.Position2 = fyne.NewPos(x, oy+oh)
		g.overlay.Objects = append(g.overlay.Objects, ln)
	}
	for r := 0; r <= g.rows; r++ {
		y := oy + float32(r)*oh/float32(g.rows)
		ln := canvas.NewLine(lineColor)
		ln.StrokeWidth = 1
		ln.Position1 = fyne.NewPos(ox, y)
		ln.Position2 = fyne.NewPos(ox+ow, y)
		g.overlay.Objects = append(g.overlay.Objects, ln)
	}
	g.overlay.Refresh()
}

func (g *CollectionGridView) cellAt(pos fyne.Position) (row, col int, ok bool) {
	ox, oy, ow, oh := g.imageContentRect()
	if ow <= 0 || oh <= 0 || g.rows < 1 || g.cols < 1 {
		return 0, 0, false
	}
	if pos.X < ox || pos.Y < oy || pos.X >= ox+ow || pos.Y >= oy+oh {
		return 0, 0, false
	}
	col = int((pos.X-ox)/ow*float32(g.cols)) + 1
	row = int((pos.Y-oy)/oh*float32(g.rows)) + 1
	if col < 1 {
		col = 1
	}
	if row < 1 {
		row = 1
	}
	if col > g.cols {
		col = g.cols
	}
	if row > g.rows {
		row = g.rows
	}
	return row, col, true
}

func (g *CollectionGridView) applyDrag(row, col int) {
	g.SetSelection(g.dragR, g.dragC, row, col)
	if g.OnSelectionChanged != nil {
		g.OnSelectionChanged(g.r1, g.c1, g.r2, g.c2)
	}
}

func (g *CollectionGridView) isZoomed() bool {
	return g.transform.Zoom > 1.01
}

func (g *CollectionGridView) Scrolled(ev *fyne.ScrollEvent) {
	if g.img.Image == nil || ev == nil {
		return
	}
	factor := ScrollZoomFactor(ev.Scrolled.DY)
	if factor == 1 {
		return
	}
	g.transform = ZoomImageAtCursor(g.Size(), g.imageSize(), g.transform, ev.Position, factor)
	g.layoutImage()
	g.rebuildOverlay()
	g.Refresh()
}

func (g *CollectionGridView) MouseDown(ev *desktop.MouseEvent) {
	if ev.Button != desktop.MouseButtonPrimary {
		return
	}
	if g.isZoomed() {
		g.panning = true
		g.panStart = ev.Position
		g.panBaseX = g.transform.PanX
		g.panBaseY = g.transform.PanY
		return
	}
	if !g.selectable {
		return
	}
	row, col, ok := g.cellAt(ev.Position)
	if !ok {
		return
	}
	g.dragging = true
	g.dragR, g.dragC = row, col
	g.applyDrag(row, col)
}

func (g *CollectionGridView) MouseUp(*desktop.MouseEvent) {
	g.dragging = false
	g.panning = false
}

func (g *CollectionGridView) MouseIn(*desktop.MouseEvent) {}
func (g *CollectionGridView) MouseOut()                   {}

func (g *CollectionGridView) MouseMoved(ev *desktop.MouseEvent) {
	if g.panning {
		g.transform.PanX = g.panBaseX + (ev.Position.X - g.panStart.X)
		g.transform.PanY = g.panBaseY + (ev.Position.Y - g.panStart.Y)
		g.transform = ClampImagePan(g.Size(), g.imageSize(), g.transform)
		g.layoutImage()
		g.rebuildOverlay()
		g.Refresh()
		return
	}
	if !g.selectable || !g.dragging {
		return
	}
	row, col, ok := g.cellAt(ev.Position)
	if !ok {
		return
	}
	g.applyDrag(row, col)
}

type collectionGridRenderer struct {
	grid    *CollectionGridView
	root    *fyne.Container
	objects []fyne.CanvasObject
}

func (r *collectionGridRenderer) Layout(size fyne.Size) {
	r.root.Resize(size)
	r.grid.layoutImage()
}

func (r *collectionGridRenderer) MinSize() fyne.Size {
	return r.grid.MinSize()
}

func (r *collectionGridRenderer) Refresh() {
	r.grid.layoutImage()
	r.grid.rebuildOverlay()
	canvas.Refresh(r.root)
}

func (r *collectionGridRenderer) Objects() []fyne.CanvasObject { return r.objects }
func (r *collectionGridRenderer) Destroy()                     {}

var (
	_ desktop.Mouseable = (*CollectionGridView)(nil)
	_ desktop.Hoverable = (*CollectionGridView)(nil)
	_ fyne.Scrollable   = (*CollectionGridView)(nil)
)
