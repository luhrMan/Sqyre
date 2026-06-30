package recording

import (
	"errors"
	"image"
	"image/color"
	"image/draw"
	"log"
	"sync"
	"time"

	"Sqyre/internal/screen"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

var errCaptureNil = errors.New("screen capture returned nil image")

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

func screenPxToCanvas(win fyne.Window, deltaPx int) float32 {
	scale := win.Canvas().Scale()
	if scale <= 0 {
		scale = 1
	}
	return float32(deltaPx) / scale
}

type fyneDesktopOverlay struct {
	win          fyne.Window
	desktopBounds image.Rectangle
	bgImage      *canvas.Image
	widthPx      int
	heightPx     int
	selLayout    *selectionRectLayout
	selRefresher interface{ Refresh() }
	stopPosition chan struct{}
}

func captureVirtualDesktop(vb image.Rectangle) (image.Image, error) {
	img, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		return nil, err
	}
	if img == nil {
		return nil, errCaptureNil
	}
	b := img.Bounds()
	rgba := image.NewRGBA(image.Rect(0, 0, b.Dx(), b.Dy()))
	draw.Draw(rgba, rgba.Bounds(), img, b.Min, draw.Src)
	return rgba, nil
}

func (o *fyneDesktopOverlay) resizeBackground() {
	if o.bgImage == nil {
		return
	}
	lw := screenPxToCanvas(o.win, o.widthPx)
	lh := screenPxToCanvas(o.win, o.heightPx)
	o.bgImage.SetMinSize(fyne.NewSize(lw, lh))
	o.bgImage.Refresh()
}

func (o *fyneDesktopOverlay) reposition() {
	positionFyneOverlayWindow(o.win, o.desktopBounds)
	o.resizeBackground()
}

// startPositionLoop keeps the overlay pinned to desktopBounds. Fyne/GLFW may
// reset window geometry after Show(); RunNative positioning is also queued
// asynchronously, so we retry until the overlay is dismissed.
func (o *fyneDesktopOverlay) startPositionLoop() {
	services.GoSafe(func() {
		delays := []time.Duration{0, 50, 100, 200, 400, 800, 1200}
		for _, d := range delays {
			select {
			case <-o.stopPosition:
				return
			case <-time.After(d):
				fyne.Do(o.reposition)
			}
		}
		ticker := time.NewTicker(250 * time.Millisecond)
		defer ticker.Stop()
		for {
			select {
			case <-o.stopPosition:
				return
			case <-ticker.C:
				fyne.Do(o.reposition)
			}
		}
	})
}

func showFullScreenOverlay(withSelectionRect bool, onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	app := fyne.CurrentApp()
	if app == nil {
		return func() {}, func(int, int, int, int) {}
	}

	vb := screen.VirtualBounds()
	if vb.Empty() || vb.Dx() <= 0 || vb.Dy() <= 0 {
		log.Printf("overlay: invalid virtual bounds %v", vb)
		return func() {}, func(int, int, int, int) {}
	}

	captureImg, err := captureVirtualDesktop(vb)
	if err != nil {
		log.Printf("overlay: virtual desktop capture failed: %v", err)
		return func() {}, func(int, int, int, int) {}
	}

	w, h := vb.Dx(), vb.Dy()
	win := app.NewWindow("")
	win.SetPadded(false)
	win.SetFixedSize(true)

	bgImage := canvas.NewImageFromImage(captureImg)
	bgImage.FillMode = canvas.ImageFillStretch
	bgImage.SetMinSize(fyne.NewSize(float32(w), float32(h)))

	var selLayout *selectionRectLayout
	var selRefresher interface{ Refresh() }
	var selectionLayer fyne.CanvasObject
	if withSelectionRect {
		selLayout = &selectionRectLayout{}
		selRect := canvas.NewRectangle(color.Transparent)
		selRect.StrokeColor = color.NRGBA{R: 255, G: 200, B: 0, A: 255}
		selRect.StrokeWidth = 2
		selContainer := container.New(selLayout, selRect)
		selectionLayer = selContainer
		selRefresher = selContainer
	} else {
		selectionLayer = layout.NewSpacer()
	}

	var stack fyne.CanvasObject
	if onMouseDown != nil {
		stack = container.NewMax(bgImage, selectionLayer, newRecordingMouseLayer(onMouseDown))
	} else {
		stack = container.NewMax(bgImage, selectionLayer)
	}
	win.SetContent(stack)

	overlay := &fyneDesktopOverlay{
		win:           win,
		desktopBounds: vb,
		bgImage:       bgImage,
		widthPx:       w,
		heightPx:      h,
		selLayout:     selLayout,
		selRefresher:  selRefresher,
		stopPosition:  make(chan struct{}),
	}

	var dismissOnce sync.Once
	var closedOnce sync.Once
	dismiss = func() {
		dismissOnce.Do(func() {
			close(overlay.stopPosition)
			win.Close()
		})
	}
	if onClosed != nil {
		win.SetOnClosed(func() { closedOnce.Do(onClosed) })
	}
	win.Canvas().SetOnTypedKey(func(e *fyne.KeyEvent) {
		if e.Name == fyne.KeyEscape {
			dismiss()
		}
	})

	win.Show()
	overlay.startPositionLoop()
	win.RequestFocus()

	if withSelectionRect && selLayout != nil && selRefresher != nil {
		setSelectionRect = func(leftX, topY, rightX, bottomY int) {
			fyne.Do(func() {
				if local, ok := clipRectToDesktop(leftX, topY, rightX, bottomY, vb); ok {
					selLayout.leftX = screenPxToCanvas(win, local.Min.X)
					selLayout.topY = screenPxToCanvas(win, local.Min.Y)
					selLayout.rightX = screenPxToCanvas(win, local.Max.X)
					selLayout.bottomY = screenPxToCanvas(win, local.Max.Y)
				} else {
					selLayout.leftX, selLayout.topY, selLayout.rightX, selLayout.bottomY = 0, 0, 0, 0
				}
				selRefresher.Refresh()
			})
		}
	} else {
		setSelectionRect = func(int, int, int, int) {}
	}

	return dismiss, setSelectionRect
}

func clipRectToDesktop(leftX, topY, rightX, bottomY int, vb image.Rectangle) (image.Rectangle, bool) {
	if leftX > rightX {
		leftX, rightX = rightX, leftX
	}
	if topY > bottomY {
		topY, bottomY = bottomY, topY
	}
	intersect := image.Rect(leftX, topY, rightX, bottomY).Intersect(vb)
	if intersect.Empty() {
		return image.Rectangle{}, false
	}
	return image.Rect(
		intersect.Min.X-vb.Min.X,
		intersect.Min.Y-vb.Min.Y,
		intersect.Max.X-vb.Min.X,
		intersect.Max.Y-vb.Min.Y,
	), true
}

// clipRectToMonitor maps absolute desktop coordinates to the intersection within bounds.
func clipRectToMonitor(leftX, topY, rightX, bottomY int, bounds image.Rectangle) (localLeft, localTop, localRight, localBottom int, ok bool) {
	if leftX > rightX {
		leftX, rightX = rightX, leftX
	}
	if topY > bottomY {
		topY, bottomY = bottomY, topY
	}
	intersect := image.Rect(leftX, topY, rightX, bottomY).Intersect(bounds)
	if intersect.Empty() {
		return 0, 0, 0, 0, false
	}
	return intersect.Min.X, intersect.Min.Y, intersect.Max.X, intersect.Max.Y, true
}

func ShowRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func() {
	dismiss, _ := showFullScreenOverlay(false, onClosed, onMouseDown)
	return dismiss
}

func ShowSearchAreaRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	return showFullScreenOverlay(true, onClosed, onMouseDown)
}
