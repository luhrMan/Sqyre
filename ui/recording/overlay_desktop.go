//go:build !js

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
	"github.com/go-vgo/robotgo"
)

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
	if bgImage != nil {
		lw := screenPxToCanvas(win, w)
		lh := screenPxToCanvas(win, h)
		bgImage.SetMinSize(fyne.NewSize(lw, lh))
		bgImage.Refresh()
	}

	if withSelectionRect && selLayout != nil && selectionLayerRefresher != nil {
		setSelectionRect = func(leftX, topY, rightX, bottomY int) {
			fyne.Do(func() {
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
