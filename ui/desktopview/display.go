// Package desktopview maps absolute virtual-desktop pixels to Fyne canvas coordinates
// for screen captures shown in the UI (recording overlay and editor previews).
package desktopview

import (
	"image"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
)

// CanvasScale returns the Fyne canvas device scale for win, defaulting to 1.
func CanvasScale(win fyne.Window) float32 {
	if win == nil {
		return 1
	}
	scale := win.Canvas().Scale()
	if scale <= 0 {
		return 1
	}
	return scale
}

// PhysicalToCanvas converts a physical pixel delta to Fyne canvas logical units.
func PhysicalToCanvas(win fyne.Window, deltaPx int) float32 {
	return float32(deltaPx) / CanvasScale(win)
}

// AbsoluteToCanvas maps absolute desktop coordinates to canvas-local coordinates
// using virtualBounds as the origin (same space as robotgo.Location / CaptureImg).
func AbsoluteToCanvas(win fyne.Window, absX, absY int, virtualBounds image.Rectangle) (float32, float32) {
	return PhysicalToCanvas(win, absX-virtualBounds.Min.X), PhysicalToCanvas(win, absY-virtualBounds.Min.Y)
}

// ClipAbsoluteRectToVirtualLocal intersects an absolute desktop rectangle with
// virtualBounds and returns coordinates relative to virtualBounds.Min.
func ClipAbsoluteRectToVirtualLocal(leftX, topY, rightX, bottomY int, virtualBounds image.Rectangle) (image.Rectangle, bool) {
	if leftX > rightX {
		leftX, rightX = rightX, leftX
	}
	if topY > bottomY {
		topY, bottomY = bottomY, topY
	}
	intersect := image.Rect(leftX, topY, rightX, bottomY).Intersect(virtualBounds)
	if intersect.Empty() {
		return image.Rectangle{}, false
	}
	return image.Rect(
		intersect.Min.X-virtualBounds.Min.X,
		intersect.Min.Y-virtualBounds.Min.Y,
		intersect.Max.X-virtualBounds.Min.X,
		intersect.Max.Y-virtualBounds.Min.Y,
	), true
}

// AbsoluteRectToCanvas maps an absolute desktop rectangle to canvas-local fyne coords.
func AbsoluteRectToCanvas(win fyne.Window, leftX, topY, rightX, bottomY int, virtualBounds image.Rectangle) (left, top, right, bottom float32, ok bool) {
	local, ok := ClipAbsoluteRectToVirtualLocal(leftX, topY, rightX, bottomY, virtualBounds)
	if !ok {
		return 0, 0, 0, 0, false
	}
	return PhysicalToCanvas(win, local.Min.X), PhysicalToCanvas(win, local.Min.Y),
		PhysicalToCanvas(win, local.Max.X), PhysicalToCanvas(win, local.Max.Y), true
}

// OverlaySnapshotFill stretches the capture to the overlay window bounds so the
// full monitor snapshot is visible without letterboxing.
const OverlaySnapshotFill = canvas.ImageFillStretch

// PreviewSnapshotFill is the fill mode for editor preview thumbnails: aspect ratio
// is preserved inside the preview panel.
const PreviewSnapshotFill = canvas.ImageFillContain

// NewOverlaySnapshotImage creates a background image for a full-desktop recording overlay.
func NewOverlaySnapshotImage(capture image.Image, win fyne.Window, widthPx, heightPx int) *canvas.Image {
	img := canvas.NewImageFromImage(capture)
	img.FillMode = OverlaySnapshotFill
	ResizeOverlaySnapshot(img, win, widthPx, heightPx)
	return img
}

// ResizeOverlaySnapshot updates an overlay snapshot to match the window canvas scale.
// The overlay window is sized in physical pixels; Fyne layout uses logical units.
func ResizeOverlaySnapshot(img *canvas.Image, win fyne.Window, widthPx, heightPx int) {
	if img == nil {
		return
	}
	img.SetMinSize(fyne.NewSize(PhysicalToCanvas(win, widthPx), PhysicalToCanvas(win, heightPx)))
	img.Refresh()
}
