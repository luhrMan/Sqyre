package ui

import (
	"fmt"
	"image"
	"math"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"golang.org/x/image/draw"
)

const (
	editorZoomStep = 1.25
	editorZoomMin  = 0.25
	editorZoomMax  = 8.0
)

// editorPreviewZoom adds zoom in / zoom out / reset controls and a scrollable
// preview, following the toolbar pattern used in github.com/Palexer/image-viewer.
type editorPreviewZoom struct {
	img       *canvas.Image
	scroll    *container.Scroll
	zoomLabel *widget.Label
	zoomIn    *widget.Button
	zoomOut   *widget.Button
	zoomReset *widget.Button
	root      fyne.CanvasObject

	zoomLevel float64
	baseImage image.Image
	minW      float32
	minH      float32
}

func newEditorPreviewZoom(minW, minH float32) *editorPreviewZoom {
	z := &editorPreviewZoom{
		minW:      minW,
		minH:      minH,
		zoomLevel: 1,
	}
	z.img = canvas.NewImageFromImage(nil)
	z.img.FillMode = canvas.ImageFillContain
	z.img.SetMinSize(fyne.NewSize(minW, minH))

	z.scroll = container.NewScroll(z.img)
	z.scroll.SetMinSize(fyne.NewSize(minW, minH))

	z.zoomLabel = widget.NewLabel("")
	z.zoomIn = widget.NewButtonWithIcon("", theme.ZoomInIcon(), z.zoomInPressed)
	z.zoomOut = widget.NewButtonWithIcon("", theme.ZoomOutIcon(), z.zoomOutPressed)
	z.zoomReset = widget.NewButtonWithIcon("", theme.ZoomFitIcon(), z.zoomResetPressed)

	toolbar := container.NewHBox(layout.NewSpacer(), z.zoomLabel, z.zoomReset, z.zoomOut, z.zoomIn)
	z.root = container.NewBorder(nil, toolbar, nil, nil, z.scroll)

	z.syncZoomButtons()
	return z
}

func (z *editorPreviewZoom) Content() fyne.CanvasObject { return z.root }

func (z *editorPreviewZoom) SetImage(img image.Image) {
	z.baseImage = img
	z.img.Resource = nil
	if img != nil {
		z.zoomLevel = 1
	}
	if img == nil {
		z.img.Image = nil
		z.zoomLevel = 1
		z.img.FillMode = canvas.ImageFillContain
		z.img.SetMinSize(fyne.NewSize(z.minW, z.minH))
		z.syncZoomButtons()
		z.img.Refresh()
		return
	}
	z.applyZoom()
}

func (z *editorPreviewZoom) Clear() {
	z.SetImage(nil)
}

func (z *editorPreviewZoom) SetBrokenIcon() {
	z.baseImage = nil
	z.zoomLevel = 1
	z.img.Image = nil
	z.img.Resource = theme.BrokenImageIcon()
	z.img.FillMode = canvas.ImageFillContain
	z.img.SetMinSize(fyne.NewSize(z.minW, z.minH))
	z.syncZoomButtons()
	z.img.Refresh()
}

func (z *editorPreviewZoom) zoomInPressed() {
	if z.baseImage == nil {
		return
	}
	z.zoomLevel = math.Min(editorZoomMax, z.zoomLevel*editorZoomStep)
	z.applyZoom()
}

func (z *editorPreviewZoom) zoomOutPressed() {
	if z.baseImage == nil {
		return
	}
	z.zoomLevel = math.Max(editorZoomMin, z.zoomLevel/editorZoomStep)
	z.applyZoom()
}

func (z *editorPreviewZoom) zoomResetPressed() {
	if z.baseImage == nil {
		return
	}
	z.zoomLevel = 1
	z.applyZoom()
}

func (z *editorPreviewZoom) applyZoom() {
	if z.baseImage == nil {
		return
	}

	b := z.baseImage.Bounds()
	sw, sh := float64(b.Dx()), float64(b.Dy())
	if sw < 1 || sh < 1 {
		return
	}

	baseFit := math.Min(float64(z.minW)/sw, float64(z.minH)/sh)
	scale := baseFit * z.zoomLevel

	dw := int(math.Max(1, math.Round(sw*scale)))
	dh := int(math.Max(1, math.Round(sh*scale)))

	dst := image.NewRGBA(image.Rect(0, 0, dw, dh))
	draw.CatmullRom.Scale(dst, dst.Bounds(), z.baseImage, b, draw.Over, nil)

	z.img.Resource = nil
	z.img.FillMode = canvas.ImageFillOriginal
	z.img.Image = dst
	z.img.Resize(fyne.NewSize(float32(dw), float32(dh)))
	z.syncZoomButtons()
	z.img.Refresh()
}

func (z *editorPreviewZoom) syncZoomButtons() {
	if z.baseImage == nil {
		z.zoomLabel.SetText("")
		z.zoomIn.Disable()
		z.zoomOut.Disable()
		z.zoomReset.Disable()
		return
	}

	z.zoomLabel.SetText(fmt.Sprintf("%.0f%%", z.zoomLevel*100))
	z.zoomReset.Enable()
	z.zoomIn.Enable()
	z.zoomOut.Enable()

	if z.zoomLevel >= editorZoomMax-1e-6 {
		z.zoomIn.Disable()
	}
	if z.zoomLevel <= editorZoomMin+1e-6 {
		z.zoomOut.Disable()
	}
}
