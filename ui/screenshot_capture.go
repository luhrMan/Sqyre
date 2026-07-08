package ui

import (
	"bytes"
	"fmt"
	"image"
	"image/draw"
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
	EnsureDataEditor()
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

// DecodePNG decodes PNG bytes for tests.
func DecodePNG(data []byte) (image.Image, error) {
	img, err := png.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, fmt.Errorf("decode png: %w", err)
	}
	return img, nil
}

// CompositePNGOver draws overlay centered on base.
func CompositePNGOver(basePNG, overlayPNG []byte) ([]byte, error) {
	base, err := png.Decode(bytes.NewReader(basePNG))
	if err != nil {
		return nil, fmt.Errorf("decode base png: %w", err)
	}
	overlay, err := png.Decode(bytes.NewReader(overlayPNG))
	if err != nil {
		return nil, fmt.Errorf("decode overlay png: %w", err)
	}
	bounds := base.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, base, image.Point{}, draw.Src)
	dx := (bounds.Dx() - overlay.Bounds().Dx()) / 2
	dy := (bounds.Dy() - overlay.Bounds().Dy()) / 2
	overlayRect := overlay.Bounds().Add(image.Pt(dx, dy))
	draw.Draw(out, overlayRect, overlay, image.Point{}, draw.Over)
	var buf bytes.Buffer
	if err := png.Encode(&buf, out); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
