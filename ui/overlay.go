package ui

import (
	"image/color"
	"log"

	"Squire/internal/config"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

// selectionRectLayout positions a single child (the selection rectangle) at
// (leftX, topY) with size (rightX-leftX, bottomY-topY). Zero size hides it.
type selectionRectLayout struct {
	leftX, topY, rightX, bottomY int
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
	objects[0].Move(fyne.NewPos(float32(s.leftX), float32(s.topY)))
	objects[0].Resize(fyne.NewSize(float32(w), float32(h)))
}

func (s *selectionRectLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(0, 0)
}

// showFullScreenOverlay captures the current screen, then creates a full-screen
// overlay showing that screenshot with centered instruction lines. It returns a
// dismiss function that safely closes the overlay on the main UI thread. If
// withSelectionRect is true, it also returns setSelectionRect to draw/update a
// rectangle from (leftX, topY) to (rightX, bottomY).
func showFullScreenOverlay(lines []string, withSelectionRect bool) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	app := fyne.CurrentApp()
	if app == nil {
		return func() {}, func(int, int, int, int) {}
	}

	w, h := config.MonitorWidth, config.MonitorHeight
	captureImg, err := robotgo.CaptureImg(0, 0, w, h)
	if err != nil || captureImg == nil {
		log.Printf("overlay: screen capture failed: %v; using blank overlay", err)
	}

	win := app.NewWindow("")
	win.SetFullScreen(true)
	win.SetPadded(false)

	var bg fyne.CanvasObject
	if captureImg != nil {
		img := canvas.NewImageFromImage(captureImg)
		img.FillMode = canvas.ImageFillStretch
		img.SetMinSize(fyne.NewSize(float32(w), float32(h)))
		bg = img
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

	dismiss = func() {
		fyne.Do(func() {
			win.Close()
		})
	}

	if withSelectionRect && selLayout != nil && selectionLayerRefresher != nil {
		setSelectionRect = func(leftX, topY, rightX, bottomY int) {
			fyne.Do(func() {
				selLayout.leftX = leftX
				selLayout.topY = topY
				selLayout.rightX = rightX
				selLayout.bottomY = bottomY
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
