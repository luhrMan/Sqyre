//go:build js

package recording

import (
	"image/color"

	"Sqyre/internal/screen"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
)

func showFullScreenOverlay(withSelectionRect bool, onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	app := fyne.CurrentApp()
	if app == nil {
		return func() {}, func(int, int, int, int) {}
	}

	absBounds := screen.DisplayBoundsAbs(0)
	w, h := absBounds.Dx(), absBounds.Dy()
	if w <= 0 || h <= 0 {
		vb := screen.VirtualBounds()
		w, h = vb.Dx(), vb.Dy()
	}
	originX, originY := absBounds.Min.X, absBounds.Min.Y

	win := app.NewWindow("")
	win.SetFullScreen(true)
	win.SetPadded(false)

	bg := canvas.NewRectangle(color.NRGBA{R: 20, G: 24, B: 32, A: 220})
	hint := canvas.NewText("Screen capture is not available in the browser — Esc to close", color.NRGBA{R: 200, G: 200, B: 200, A: 255})
	hint.TextSize = 16
	bgStack := container.NewStack(bg, container.NewCenter(hint))

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
		stack = container.NewMax(bgStack, selectionLayer, newRecordingMouseLayer(onMouseDown))
	} else {
		stack = container.NewMax(bgStack, selectionLayer)
	}
	win.SetContent(stack)
	win.Canvas().SetOnTypedKey(func(e *fyne.KeyEvent) {
		if e.Name == fyne.KeyEscape {
			dismiss()
		}
	})
	win.Show()
	win.RequestFocus()

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
