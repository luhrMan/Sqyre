package recording

import (
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
)

// recordingMouseLayer sits on top of the overlay stack and receives mouse clicks
// via Fyne (overlay window has focus) instead of a global low-level hook.
type recordingMouseLayer struct {
	widget.BaseWidget
	onMouseDown func(*desktop.MouseEvent)
}

func newRecordingMouseLayer(onMouseDown func(*desktop.MouseEvent)) *recordingMouseLayer {
	r := &recordingMouseLayer{onMouseDown: onMouseDown}
	r.ExtendBaseWidget(r)
	return r
}

func (r *recordingMouseLayer) MouseDown(ev *desktop.MouseEvent) {
	if r.onMouseDown != nil {
		r.onMouseDown(ev)
	}
}

func (r *recordingMouseLayer) MouseUp(*desktop.MouseEvent) {}

func (r *recordingMouseLayer) CreateRenderer() fyne.WidgetRenderer {
	rect := canvas.NewRectangle(color.Transparent)
	return widget.NewSimpleRenderer(rect)
}

// selectionRectLayout positions a single child (the selection rectangle) at
// (leftX, topY) with size (rightX-leftX, bottomY-topY) in Fyne canvas units.
type selectionRectLayout struct {
	leftX, topY, rightX, bottomY float32
}

func (s *selectionRectLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	if len(objects) < 1 {
		return
	}
	w := s.rightX - s.leftX
	h := s.bottomY - s.topY
	if w <= 0 || h <= 0 {
		objects[0].Resize(fyne.NewSize(0, 0))
		objects[0].Move(fyne.NewPos(0, 0))
		return
	}
	objects[0].Move(fyne.NewPos(s.leftX, s.topY))
	objects[0].Resize(fyne.NewSize(w, h))
}

func (s *selectionRectLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(0, 0)
}

// screenPxToCanvas converts a delta in physical screen pixels to Fyne canvas coordinates.
func screenPxToCanvas(win fyne.Window, deltaPx int) float32 {
	scale := win.Canvas().Scale()
	if scale <= 0 {
		scale = 1
	}
	return float32(deltaPx) / scale
}

// ShowRecordingOverlay opens a full-screen overlay for coordinate recording.
func ShowRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func() {
	dismiss, _ := showFullScreenOverlay(false, onClosed, onMouseDown)
	return dismiss
}

// ShowSearchAreaRecordingOverlay opens an overlay with a live selection rectangle.
func ShowSearchAreaRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	return showFullScreenOverlay(true, onClosed, onMouseDown)
}
