package custom_widgets

import (
	"math"

	"fyne.io/fyne/v2"
)

const (
	imageZoomMin       = 0.5
	imageZoomMax       = 16.0
	imageZoomWheelStep = 0.07 // 7% per wheel notch
	imagePanEdgePadding float32 = 32 // extra pan range so image edges are not flush with the viewport
)

// ImageViewTransform holds zoom and pan state for a fit-to-viewport image preview.
// Zoom 1.0 means the image fits inside the viewport; values above enlarge it.
type ImageViewTransform struct {
	Zoom float32
	PanX float32
	PanY float32
}

// ResetImageViewTransform returns the default fit-to-viewport transform.
func ResetImageViewTransform() ImageViewTransform {
	return ImageViewTransform{Zoom: 1}
}

// ImagePixelSize returns the logical size of an image with the given bounds.
func ImagePixelSize(width, height int) fyne.Size {
	return fyne.NewSize(float32(width), float32(height))
}

func fitImageScale(viewport, imageSize fyne.Size) float32 {
	if viewport.Width <= 0 || viewport.Height <= 0 || imageSize.Width <= 0 || imageSize.Height <= 0 {
		return 1
	}
	return float32(math.Min(float64(viewport.Width/imageSize.Width), float64(viewport.Height/imageSize.Height)))
}

// ImageContentRect returns the displayed image rectangle inside the viewport.
func ImageContentRect(viewport, imageSize fyne.Size, t ImageViewTransform) (x, y, w, h float32) {
	if viewport.Width <= 0 || viewport.Height <= 0 {
		return 0, 0, 0, 0
	}
	if imageSize.Width <= 0 || imageSize.Height <= 0 {
		return 0, 0, viewport.Width, viewport.Height
	}
	scale := fitImageScale(viewport, imageSize) * t.Zoom
	w = imageSize.Width * scale
	h = imageSize.Height * scale
	x = (viewport.Width-w)/2 + t.PanX
	y = (viewport.Height-h)/2 + t.PanY
	return x, y, w, h
}

// ScrollZoomFactor converts a wheel delta into a multiplicative zoom factor.
// Fyne scales raw wheel deltas by scrollSpeed (25 on Linux), so we use direction
// only and apply a fixed step per notch for consistent, smooth zooming.
func ScrollZoomFactor(deltaY float32) float32 {
	if deltaY == 0 {
		return 1
	}
	if deltaY > 0 {
		return 1 + imageZoomWheelStep
	}
	return 1 / (1 + imageZoomWheelStep)
}

// ZoomImageAtCursor updates transform to zoom around cursor, returning the new transform.
func ZoomImageAtCursor(viewport, imageSize fyne.Size, t ImageViewTransform, cursor fyne.Position, factor float32) ImageViewTransform {
	if factor <= 0 || imageSize.Width <= 0 || imageSize.Height <= 0 {
		return t
	}
	x, y, w, h := ImageContentRect(viewport, imageSize, t)
	if w <= 0 || h <= 0 {
		return t
	}
	u := (cursor.X - x) / w
	v := (cursor.Y - y) / h

	newZoom := clampImageZoom(t.Zoom * factor)
	if newZoom == t.Zoom {
		return t
	}
	t.Zoom = newZoom

	nx, ny, nw, nh := ImageContentRect(viewport, imageSize, t)
	_ = nh
	t.PanX += (cursor.X - u*nw) - nx
	t.PanY += (cursor.Y - v*nh) - ny
	return ClampImagePan(viewport, imageSize, t)
}

// ClampImagePan keeps the image in view while allowing a small margin past each edge when zoomed.
func ClampImagePan(viewport, imageSize fyne.Size, t ImageViewTransform) ImageViewTransform {
	x, y, w, h := ImageContentRect(viewport, imageSize, t)
	pad := imagePanEdgePadding
	if w <= viewport.Width {
		t.PanX = 0
	} else {
		minX := viewport.Width - w - pad
		maxX := pad
		if x < minX {
			t.PanX += minX - x
		}
		if x > maxX {
			t.PanX += maxX - x
		}
	}
	if h <= viewport.Height {
		t.PanY = 0
	} else {
		minY := viewport.Height - h - pad
		maxY := pad
		if y < minY {
			t.PanY += minY - y
		}
		if y > maxY {
			t.PanY += maxY - y
		}
	}
	return t
}

func clampImageZoom(z float32) float32 {
	if z < imageZoomMin {
		return imageZoomMin
	}
	if z > imageZoomMax {
		return imageZoomMax
	}
	return z
}
