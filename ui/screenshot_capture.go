package ui

import (
	"bytes"
	"image/png"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
)

const screenshotWindowW, screenshotWindowH = 1000, 500

// MacroScreenForScreenshot returns the macro editor layout for docs/tests.
func MacroScreenForScreenshot(u *Ui) fyne.CanvasObject {
	return u.constructMacroUi()
}

// EditorScreenForScreenshot returns the data editor layout for docs/tests.
func EditorScreenForScreenshot(u *Ui) fyne.CanvasObject {
	return u.EditorUi.CanvasObject
}

// PrepareWindowForCapture sizes the window and refreshes the live canvas tree.
func PrepareWindowForCapture(w fyne.Window) {
	size := fyne.NewSize(screenshotWindowW, screenshotWindowH)
	w.Resize(size)
	w.Show()
	if content := w.Canvas().Content(); content != nil {
		content.Refresh()
		w.Canvas().Refresh(content)
	}
}

// CaptureWindowPNG captures the rendered window canvas (widgets stay on the canvas).
func CaptureWindowPNG(w fyne.Window) ([]byte, error) {
	PrepareWindowForCapture(w)
	img := w.Canvas().Capture()
	if img == nil {
		return nil, nil
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// RenderObjectPNG renders a widget tree on a headless test window with Fyne's layout engine.
func RenderObjectPNG(obj fyne.CanvasObject, size fyne.Size) ([]byte, error) {
	w := test.NewWindow(obj)
	defer w.Close()

	canvasSize := obj.MinSize().Max(obj.Size())
	if size.Width > 0 && size.Width > canvasSize.Width {
		canvasSize.Width = size.Width
	}
	if size.Height > 0 && size.Height > canvasSize.Height {
		canvasSize.Height = size.Height
	}
	if canvasSize.Width < 200 {
		canvasSize.Width = 200
	}
	if canvasSize.Height < 100 {
		canvasSize.Height = 100
	}

	w.Resize(canvasSize)
	w.Show()
	obj.Refresh()
	w.Canvas().Refresh(obj)

	img := w.Canvas().Capture()
	if img == nil {
		return nil, nil
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
