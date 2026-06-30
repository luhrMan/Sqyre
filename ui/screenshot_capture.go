package ui

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"image/png"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/macro/actiondialog"

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

// OverlayActionDialogOnMainPNG composites a dimmed main-window capture with a centered action dialog.
func OverlayActionDialogOnMainPNG(mainPNG []byte, action actions.ActionInterface) ([]byte, error) {
	base, err := png.Decode(bytes.NewReader(mainPNG))
	if err != nil {
		return nil, fmt.Errorf("decode main window png: %w", err)
	}
	parent := fyne.NewSize(screenshotWindowW, screenshotWindowH)
	panel := actiondialog.PanelForScreenshot(action)
	panelSize := actiondialog.ScreenshotSizeOnParent(parent, action)
	panel.Resize(panelSize)
	dialogPNG, err := RenderObjectPNG(panel, panelSize)
	if err != nil {
		return nil, err
	}
	dialogImg, err := png.Decode(bytes.NewReader(dialogPNG))
	if err != nil {
		return nil, fmt.Errorf("decode dialog png: %w", err)
	}

	bounds := base.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, base, image.Point{}, draw.Src)
	scrim := image.NewUniform(color.NRGBA{R: 0, G: 0, B: 0, A: 160})
	draw.Draw(out, bounds, scrim, image.Point{}, draw.Over)

	dx := (bounds.Dx() - dialogImg.Bounds().Dx()) / 2
	dy := (bounds.Dy() - dialogImg.Bounds().Dy()) / 2
	dialogRect := dialogImg.Bounds().Add(image.Pt(dx, dy))
	draw.Draw(out, dialogRect, dialogImg, image.Point{}, draw.Over)

	var buf bytes.Buffer
	if err := png.Encode(&buf, out); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
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
