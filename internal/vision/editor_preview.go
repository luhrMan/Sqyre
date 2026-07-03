package vision

import (
	"Sqyre/internal/config"
	"Sqyre/internal/macro"
	"Sqyre/internal/screen"
	"fmt"
	"image"
	"image/color"

	"gocv.io/x/gocv"
)

const (
	editorPreviewMonitorDash = 10
	editorPreviewMonitorGap  = 6
)

// EditorPreviewMonitorOutline is the dotted monitor bezel color (matches search-area / point accent).
var EditorPreviewMonitorOutline = color.RGBA{R: 255, A: 255}

func drawPreviewDottedHLine(mat *gocv.Mat, y, x0, x1 int, c color.RGBA, thick int) {
	if x0 > x1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for x := x0; x <= x1; x += step {
		xe := min(x+editorPreviewMonitorDash-1, x1)
		gocv.Line(mat, image.Pt(x, y), image.Pt(xe, y), c, thick)
	}
}

func drawPreviewDottedVLine(mat *gocv.Mat, x, y0, y1 int, c color.RGBA, thick int) {
	if y0 > y1 {
		return
	}
	step := editorPreviewMonitorDash + editorPreviewMonitorGap
	for y := y0; y <= y1; y += step {
		ye := min(y+editorPreviewMonitorDash-1, y1)
		gocv.Line(mat, image.Point{X: x, Y: y}, image.Point{X: x, Y: ye}, c, thick)
	}
}

func drawPreviewDottedRectOutline(mat *gocv.Mat, r image.Rectangle, c color.RGBA, thick int) {
	if r.Empty() || r.Dx() <= 0 || r.Dy() <= 0 {
		return
	}
	x0, y0 := r.Min.X, r.Min.Y
	x1, y1 := r.Max.X-1, r.Max.Y-1
	if x1 < x0 || y1 < y0 {
		return
	}
	drawPreviewDottedHLine(mat, y0, x0, x1, c, thick)
	drawPreviewDottedHLine(mat, y1, x0, x1, c, thick)
	drawPreviewDottedVLine(mat, x0, y0, y1, c, thick)
	drawPreviewDottedVLine(mat, x1, y0, y1, c, thick)
}

// DrawEditorPreviewMonitorOutlines draws a dotted rectangle for each enabled monitor (clip to capture).
func DrawEditorPreviewMonitorOutlines(mat *gocv.Mat, captureBounds image.Rectangle) {
	const thick = 1
	n := screen.NumDisplays()
	for i := range n {
		if !screen.IsMonitorEnabled(i) {
			continue
		}
		b := screen.DisplayBoundsAbs(i)
		inter := b.Intersect(captureBounds)
		if inter.Empty() {
			continue
		}
		rel := image.Rect(inter.Min.X-captureBounds.Min.X, inter.Min.Y-captureBounds.Min.Y, inter.Max.X-captureBounds.Min.X, inter.Max.Y-captureBounds.Min.Y)
		drawPreviewDottedRectOutline(mat, rel, EditorPreviewMonitorOutline, thick)
	}
}

// DrawPreviewRectangle draws a solid rectangle on a preview mat.
func DrawPreviewRectangle(mat *gocv.Mat, r image.Rectangle, c color.RGBA, thick int) {
	gocv.Rectangle(mat, r, c, thick)
}

// DrawPreviewPointMarker draws a crosshair marker at center on a preview mat.
func DrawPreviewPointMarker(mat *gocv.Mat, center image.Point, c color.RGBA, thick int) {
	gocv.Circle(mat, center, 8, c, thick)
	gocv.Line(mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, c, thick)
	gocv.Line(mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, c, thick)
}

func previewCaptureBoundsForPoint(px, py int) image.Rectangle {
	vb := screen.VirtualBounds()
	half := config.EditorPreviewMinCaptureSize / 2
	desired := image.Rect(px-half, py-half, px+half, py+half)
	return shiftRectIntoVirtualBounds(desired, vb)
}

func previewCaptureBoundsForSearchArea(lx, ty, rx, by int) image.Rectangle {
	vb := screen.VirtualBounds()
	if lx > rx {
		lx, rx = rx, lx
	}
	if ty > by {
		ty, by = by, ty
	}
	area := image.Rect(lx, ty, rx, by)
	padX := max(config.EditorPreviewPadding, area.Dx()/4)
	padY := max(config.EditorPreviewPadding, area.Dy()/4)
	desired := image.Rect(lx-padX, ty-padY, rx+padX, by+padY)
	desired = expandRectToMinSize(desired, config.EditorPreviewMinCaptureSize, config.EditorPreviewMinCaptureSize)
	return shiftRectIntoVirtualBounds(desired, vb)
}

func expandRectToMinSize(r image.Rectangle, minW, minH int) image.Rectangle {
	if r.Empty() {
		return r
	}
	w := max(r.Dx(), minW)
	h := max(r.Dy(), minH)
	cx := (r.Min.X + r.Max.X) / 2
	cy := (r.Min.Y + r.Max.Y) / 2
	return image.Rect(cx-w/2, cy-h/2, cx-w/2+w, cy-h/2+h)
}

func shiftRectIntoVirtualBounds(desired, vb image.Rectangle) image.Rectangle {
	if desired.Empty() || vb.Empty() {
		return desired.Intersect(vb)
	}
	w, h := desired.Dx(), desired.Dy()
	if w <= 0 || h <= 0 {
		return image.Rectangle{}
	}
	if w >= vb.Dx() && h >= vb.Dy() {
		return vb
	}
	x0, y0 := desired.Min.X, desired.Min.Y
	if w > vb.Dx() {
		x0 = vb.Min.X
		w = vb.Dx()
	} else {
		if x0 < vb.Min.X {
			x0 = vb.Min.X
		}
		if x0+w > vb.Max.X {
			x0 = vb.Max.X - w
		}
	}
	if h > vb.Dy() {
		y0 = vb.Min.Y
		h = vb.Dy()
	} else {
		if y0 < vb.Min.Y {
			y0 = vb.Min.Y
		}
		if y0+h > vb.Max.Y {
			y0 = vb.Max.Y - h
		}
	}
	return image.Rect(x0, y0, x0+w, y0+h)
}

func captureRegionWithOverlay(captureBounds image.Rectangle, drawOverlay func(*gocv.Mat, image.Rectangle)) (image.Image, error) {
	if captureBounds.Empty() || captureBounds.Dx() <= 0 || captureBounds.Dy() <= 0 {
		return nil, fmt.Errorf("invalid capture bounds %v", captureBounds)
	}
	captureImg, err := macro.CaptureRect(captureBounds.Min.X, captureBounds.Min.Y, captureBounds.Dx(), captureBounds.Dy())
	if err != nil {
		return nil, fmt.Errorf("error capturing image: %w", err)
	}

	var out image.Image
	var matErr error
	WithOpenCV(func() {
		mat, err := gocv.ImageToMatRGB(captureImg)
		if err != nil {
			matErr = fmt.Errorf("error converting image to Mat: %w", err)
			return
		}
		defer mat.Close()

		DrawEditorPreviewMonitorOutlines(&mat, captureBounds)
		if drawOverlay != nil {
			drawOverlay(&mat, captureBounds)
		}

		out, matErr = mat.ToImage()
	})
	if matErr != nil {
		return nil, matErr
	}
	return out, nil
}

// CaptureSearchAreaPreview captures a cropped region around the search area with a rectangle overlay.
func CaptureSearchAreaPreview(lx, ty, rx, by int) (image.Image, error) {
	captureBounds := previewCaptureBoundsForSearchArea(lx, ty, rx, by)
	return captureRegionWithOverlay(captureBounds, func(mat *gocv.Mat, bounds image.Rectangle) {
		rect := image.Rect(lx-bounds.Min.X, ty-bounds.Min.Y, rx-bounds.Min.X, by-bounds.Min.Y)
		DrawPreviewRectangle(mat, rect, color.RGBA{R: 255, A: 255}, 2)
	})
}

// CapturePointPreview captures a cropped region around the point with a crosshair overlay.
func CapturePointPreview(px, py int) (image.Image, error) {
	captureBounds := previewCaptureBoundsForPoint(px, py)
	return captureRegionWithOverlay(captureBounds, func(mat *gocv.Mat, bounds image.Rectangle) {
		center := image.Point{X: px - bounds.Min.X, Y: py - bounds.Min.Y}
		DrawPreviewPointMarker(mat, center, color.RGBA{R: 255, A: 255}, 2)
	})
}

// ReadColorImage loads a color image from disk via OpenCV.
func ReadColorImage(path string) (image.Image, error) {
	var out image.Image
	var readErr error
	WithOpenCV(func() {
		mat := gocv.IMRead(path, gocv.IMReadColor)
		if mat.Empty() {
			readErr = fmt.Errorf("read image %q: empty or missing", path)
			return
		}
		defer mat.Close()
		out, readErr = mat.ToImage()
	})
	return out, readErr
}
