package ui

import (
	"image"
	"image/color"
	"log"

	"Sqyre/internal/screen"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

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
func showFullScreenOverlay(lines []string, withSelectionRect bool) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
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

	var labelObjects []fyne.CanvasObject
	for _, line := range lines {
		if line == "" {
			continue
		}
		lbl := widget.NewLabel(line)
		lbl.Alignment = fyne.TextAlignCenter
		labelObjects = append(labelObjects, lbl)
	}

	content := container.NewVBox(labelObjects...)
	centered := container.NewCenter(content)

	win.SetContent(container.NewMax(bg, selectionLayer, centered))
	win.Show()
	// Fyne sizes are canvas (logical) units; robotgo uses physical screen pixels.
	if bgImage != nil {
		lw := screenPxToCanvas(win, w)
		lh := screenPxToCanvas(win, h)
		bgImage.SetMinSize(fyne.NewSize(lw, lh))
		bgImage.Refresh()
	}

	dismiss = func() {
		fyne.Do(func() {
			win.Close()
		})
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

// ShowRecordingOverlay displays a full-screen overlay with standard
// recording instructions and returns a dismiss function.
func ShowRecordingOverlay(title, line1, line2 string) func() {
	lines := []string{title, "", line1, line2}
	dismiss, _ := showFullScreenOverlay(lines, false)
	return dismiss
}

// ShowSearchAreaRecordingOverlay is like ShowRecordingOverlay but also returns
// setSelectionRect(leftX, topY, rightX, bottomY) to draw a live selection
// rectangle from the first click to the current cursor.
func ShowSearchAreaRecordingOverlay(title, line1, line2 string) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	lines := []string{title, "", line1, line2}
	return showFullScreenOverlay(lines, true)
}
