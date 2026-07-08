// Package screenshot renders Fyne widget trees to PNG and annotates them with
// click guides for README/demo docs. It has no dependency on package ui so the
// docs tests and tooling can drive it without an import cycle.
package screenshot

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
	png, _, err := RenderObjectPNGWithAnchors(obj, size, nil)
	return png, err
}

// RenderObjectPNGWithAnchors renders obj like RenderObjectPNG, but runs resolve
// after the tree is laid out (so callers can read widget geometry via the
// driver) and returns whatever canvas-space positions it produces. The returned
// positions share the PNG's coordinate space, so they can be passed straight to
// click-guide overlays.
func RenderObjectPNGWithAnchors(obj fyne.CanvasObject, size fyne.Size, resolve func() []fyne.Position) ([]byte, []fyne.Position, error) {
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

	var anchors []fyne.Position
	if resolve != nil {
		anchors = resolve()
	}

	img := w.Canvas().Capture()
	if img == nil {
		return nil, anchors, nil
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return nil, anchors, err
	}
	return buf.Bytes(), anchors, nil
}

// AnchorCenter returns the canvas-absolute center of a laid-out widget, for use
// inside a RenderObjectPNGWithAnchors resolve callback.
func AnchorCenter(obj fyne.CanvasObject) fyne.Position {
	if obj == nil {
		return fyne.Position{}
	}
	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(obj)
	sz := obj.Size()
	return pos.Add(fyne.NewPos(sz.Width/2, sz.Height/2))
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
