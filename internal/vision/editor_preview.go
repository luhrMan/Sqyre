package vision

import (
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
		gocv.Line(mat, image.Pt(x, y), image.Pt(x, ye), c, thick)
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
func DrawEditorPreviewMonitorOutlines(mat *gocv.Mat, vb image.Rectangle) {
	const thick = 1
	n := screen.NumDisplays()
	for i := range n {
		if !screen.IsMonitorEnabled(i) {
			continue
		}
		b := screen.DisplayBoundsAbs(i)
		inter := b.Intersect(vb)
		if inter.Empty() {
			continue
		}
		rel := image.Rect(inter.Min.X-vb.Min.X, inter.Min.Y-vb.Min.Y, inter.Max.X-vb.Min.X, inter.Max.Y-vb.Min.Y)
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

// CaptureVirtualDesktopWithOverlay captures the virtual desktop and optionally draws overlays.
func CaptureVirtualDesktopWithOverlay(drawOverlay func(*gocv.Mat, image.Rectangle)) (image.Image, error) {
	captureImg, vb, err := macro.CaptureVirtualDesktop()
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

		DrawEditorPreviewMonitorOutlines(&mat, vb)
		if drawOverlay != nil {
			drawOverlay(&mat, vb)
		}

		out, matErr = mat.ToImage()
	})
	if matErr != nil {
		return nil, matErr
	}
	return out, nil
}

// CaptureSearchAreaPreview captures the virtual desktop with a search-area rectangle overlay.
func CaptureSearchAreaPreview(lx, ty, rx, by int) (image.Image, error) {
	return CaptureVirtualDesktopWithOverlay(func(mat *gocv.Mat, bounds image.Rectangle) {
		rect := image.Rect(lx-bounds.Min.X, ty-bounds.Min.Y, rx-bounds.Min.X, by-bounds.Min.Y)
		DrawPreviewRectangle(mat, rect, color.RGBA{R: 255, A: 255}, 2)
	})
}

// CapturePointPreview captures the virtual desktop with a point marker overlay.
func CapturePointPreview(px, py int) (image.Image, error) {
	return CaptureVirtualDesktopWithOverlay(func(mat *gocv.Mat, bounds image.Rectangle) {
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
