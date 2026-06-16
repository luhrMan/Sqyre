package ui

import (
	"bytes"
	"image/png"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver/software"
)

const screenshotWindowW, screenshotWindowH = 1000, 500

// PrepareWindowForCapture sizes the window and runs layout on the live canvas tree.
func PrepareWindowForCapture(w fyne.Window) {
	size := fyne.NewSize(screenshotWindowW, screenshotWindowH)
	w.Resize(size)
	w.Show()
	if content := w.Canvas().Content(); content != nil {
		_, area := w.Canvas().InteractiveArea()
		propagateLayout(content, area)
		content.Refresh()
		w.Canvas().Refresh(content)
	}
}

func propagateLayout(obj fyne.CanvasObject, size fyne.Size) {
	if obj == nil || size.Width <= 0 || size.Height <= 0 {
		return
	}
	obj.Resize(size)
	switch node := obj.(type) {
	case *fyne.Container:
		if node.Layout != nil {
			node.Layout.Layout(node.Objects, size)
		}
		for _, child := range node.Objects {
			childSize := child.Size()
			if childSize.Width <= 0 || childSize.Height <= 0 {
				childSize = child.MinSize()
			}
			propagateLayout(child, childSize)
		}
	case fyne.Widget:
		node.Refresh()
	}
}

func layoutDetached(obj fyne.CanvasObject, size fyne.Size) {
	if obj == nil || size.Width <= 0 || size.Height <= 0 {
		return
	}
	obj.Resize(size)
	switch node := obj.(type) {
	case *fyne.Container:
		if node.Layout != nil {
			node.Layout.Layout(node.Objects, size)
		}
		for _, child := range node.Objects {
			childSize := child.Size()
			if childSize.Width <= 0 || childSize.Height <= 0 {
				childSize = child.MinSize()
			}
			layoutDetached(child, childSize)
		}
	case fyne.Widget:
		r := node.CreateRenderer()
		r.Layout(size)
		for _, child := range r.Objects() {
			childSize := child.Size()
			if childSize.Width <= 0 || childSize.Height <= 0 {
				childSize = child.MinSize()
			}
			layoutDetached(child, childSize)
		}
		r.Refresh()
	}
}

// MacroScreenForScreenshot returns a detached macro editor layout for docs/tests.
func MacroScreenForScreenshot(u *Ui) fyne.CanvasObject {
	return u.constructMacroUi()
}

// EditorScreenForScreenshot returns the data editor layout for docs/tests.
func EditorScreenForScreenshot(u *Ui) fyne.CanvasObject {
	return u.EditorUi.CanvasObject
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

// RenderObjectPNG renders a standalone widget tree (never attached to another canvas).
func RenderObjectPNG(obj fyne.CanvasObject, size fyne.Size) ([]byte, error) {
	if size.Width <= 0 || size.Height <= 0 {
		size = obj.MinSize()
	}
	if size.Width < 200 {
		size.Width = 400
	}
	if size.Height < 100 {
		size.Height = 250
	}
	c := software.NewCanvas()
	c.SetPadded(false)
	c.Resize(size)
	c.SetContent(obj)
	layoutDetached(obj, size)
	img := c.Capture()
	if img == nil {
		return nil, nil
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
