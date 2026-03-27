package recording

import (
	"image"
	"image/color"
	"log"

	"Sqyre/internal/screen"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
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
// Zero size hides it.
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

// screenPxToCanvas converts a delta in physical screen pixels (robotgo space)
// to Fyne canvas coordinates for the given window.
func screenPxToCanvas(win fyne.Window, deltaPx int) float32 {
	scale := win.Canvas().Scale()
	if scale <= 0 {
		scale = 1
	}
	return float32(deltaPx) / scale
}

func (s *selectionRectLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(0, 0)
}

// showFullScreenOverlay captures the monitor under the cursor in absolute
// desktop coordinates (same space as robotgo.Location), then creates a
// full-screen overlay on that monitor. setSelectionRect expects absolute
// coordinates from the caller and maps them to overlay-local space.
func showFullScreenOverlay(withSelectionRect bool, onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	app := fyne.CurrentApp()
	if app == nil {
		return func() {}, func(int, int, int, int) {}
	}

	idx := screen.MonitorIndexForOverlay()
	absBounds := screen.DisplayBoundsAbs(idx)
	if absBounds.Empty() {
		absBounds = screen.DisplayBoundsAbs(0)
	}
	w, h := absBounds.Dx(), absBounds.Dy()
	if w <= 0 || h <= 0 {
		vb := screen.VirtualBounds()
		w, h = vb.Dx(), vb.Dy()
	}
	var captureImg image.Image
	if w <= 0 || h <= 0 || absBounds.Empty() {
		log.Printf("overlay: could not resolve display bounds; using blank overlay")
	} else {
		var err error
		captureImg, err = robotgo.CaptureImg(absBounds.Min.X, absBounds.Min.Y, w, h)
		if err != nil {
			log.Printf("overlay: screen capture failed: %v; using blank overlay", err)
			captureImg = nil
		}
	}
	originX, originY := absBounds.Min.X, absBounds.Min.Y

	win := app.NewWindow("")
	win.SetFullScreen(true)
	win.SetPadded(false)

	var bg fyne.CanvasObject
	var bgImage *canvas.Image
	if captureImg != nil {
		img := canvas.NewImageFromImage(captureImg)
		img.FillMode = canvas.ImageFillStretch
		// Physical pixel size; adjusted to canvas units after Show() when Scale() is known.
		img.SetMinSize(fyne.NewSize(float32(w), float32(h)))
		bg = img
		bgImage = img
	} else {
		bg = canvas.NewRectangle(color.NRGBA{R: 0, G: 0, B: 0, A: 25})
	}

	var selectionLayer fyne.CanvasObject
	var selLayout *selectionRectLayout
	var selectionLayerRefresher interface{ Refresh() }
	if withSelectionRect {
		selLayout = &selectionRectLayout{}
		selRect := canvas.NewRectangle(color.Transparent)
		selRect.StrokeColor = color.NRGBA{R: 255, G: 200, B: 0, A: 255}
		selRect.StrokeWidth = 2
		selContainer := container.New(selLayout, selRect)
		selectionLayer = selContainer
		selectionLayerRefresher = selContainer
	} else {
		selectionLayer = layout.NewSpacer()
	}

	dismiss = func() {
		fyne.Do(func() {
			win.Close()
		})
	}
	if onClosed != nil {
		win.SetOnClosed(onClosed)
	}
	var stack fyne.CanvasObject
	if onMouseDown != nil {
		stack = container.NewMax(bg, selectionLayer, newRecordingMouseLayer(onMouseDown))
	} else {
		stack = container.NewMax(bg, selectionLayer)
	}
	win.SetContent(stack)
	win.Canvas().SetOnTypedKey(func(e *fyne.KeyEvent) {
		if e.Name == fyne.KeyEscape {
			dismiss()
		}
	})
	win.Show()
	win.RequestFocus()
	// Fyne sizes are canvas (logical) units; robotgo uses physical screen pixels.
	if bgImage != nil {
		lw := screenPxToCanvas(win, w)
		lh := screenPxToCanvas(win, h)
		bgImage.SetMinSize(fyne.NewSize(lw, lh))
		bgImage.Refresh()
	}

	if withSelectionRect && selLayout != nil && selectionLayerRefresher != nil {
		setSelectionRect = func(leftX, topY, rightX, bottomY int) {
			fyne.Do(func() {
				// Absolute desktop coords -> monitor-local physical pixels -> Fyne canvas coords.
				selLayout.leftX = screenPxToCanvas(win, leftX-originX)
				selLayout.topY = screenPxToCanvas(win, topY-originY)
				selLayout.rightX = screenPxToCanvas(win, rightX-originX)
				selLayout.bottomY = screenPxToCanvas(win, bottomY-originY)
				selectionLayerRefresher.Refresh()
			})
		}
	} else {
		setSelectionRect = func(int, int, int, int) {}
	}

	return dismiss, setSelectionRect
}

func ShowRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func() {
	dismiss, _ := showFullScreenOverlay(false, onClosed, onMouseDown)
	return dismiss
}

func ShowSearchAreaRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	return showFullScreenOverlay(true, onClosed, onMouseDown)
}
