//go:build !sqyre_no_desktop_native

package desktop

import (
	"fmt"
	"image"
	"image/color"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

// MonitorOutline describes one display for dotted-outline preview drawing.
type MonitorOutline struct {
	AbsBounds image.Rectangle
	Enabled   bool
}

const (
	previewMonitorDash = 10
	previewMonitorGap  = 6
)

var previewMonitorOutlineColor = color.RGBA{R: 255, A: 255}

func drawPreviewDottedHLine(mat *gocv.Mat, y, x0, x1 int, c color.RGBA, thick int) {
	if x0 > x1 {
		return
	}
	step := previewMonitorDash + previewMonitorGap
	for x := x0; x <= x1; x += step {
		xe := x + previewMonitorDash - 1
		if xe > x1 {
			xe = x1
		}
		gocv.Line(mat, image.Pt(x, y), image.Pt(xe, y), c, thick)
	}
}

func drawPreviewDottedVLine(mat *gocv.Mat, x, y0, y1 int, c color.RGBA, thick int) {
	if y0 > y1 {
		return
	}
	step := previewMonitorDash + previewMonitorGap
	for y := y0; y <= y1; y += step {
		ye := y + previewMonitorDash - 1
		if ye > y1 {
			ye = y1
		}
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

func drawPreviewMonitorOutlines(mat *gocv.Mat, vb image.Rectangle, monitors []MonitorOutline) {
	const thick = 1
	for _, m := range monitors {
		if !m.Enabled {
			continue
		}
		b := m.AbsBounds
		inter := b.Intersect(vb)
		if inter.Empty() {
			continue
		}
		rel := image.Rect(inter.Min.X-vb.Min.X, inter.Min.Y-vb.Min.Y, inter.Max.X-vb.Min.X, inter.Max.Y-vb.Min.Y)
		drawPreviewDottedRectOutline(mat, rel, previewMonitorOutlineColor, thick)
	}
}

// SearchAreaPreviewImage captures the virtual rectangle vb and draws monitor outlines plus a red selection rect.
func SearchAreaPreviewImage(vb image.Rectangle, lx, ty, rx, by int, monitors []MonitorOutline) (image.Image, error) {
	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		return nil, err
	}
	if captureImg == nil {
		return nil, fmt.Errorf("capture returned nil image")
	}
	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		return nil, err
	}
	defer mat.Close()
	drawPreviewMonitorOutlines(&mat, vb, monitors)
	rect := image.Rect(lx-vb.Min.X, ty-vb.Min.Y, rx-vb.Min.X, by-vb.Min.Y)
	gocv.Rectangle(&mat, rect, color.RGBA{R: 255, A: 255}, 2)
	out, err := mat.ToImage()
	if err != nil {
		return nil, err
	}
	return out, nil
}

// PointPreviewImage captures vb and draws a crosshair at absolute px, py.
func PointPreviewImage(vb image.Rectangle, px, py int, monitors []MonitorOutline) (image.Image, error) {
	captureImg, err := robotgo.CaptureImg(vb.Min.X, vb.Min.Y, vb.Dx(), vb.Dy())
	if err != nil {
		return nil, err
	}
	if captureImg == nil {
		return nil, fmt.Errorf("capture returned nil image")
	}
	mat, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		return nil, err
	}
	defer mat.Close()
	drawPreviewMonitorOutlines(&mat, vb, monitors)
	center := image.Point{X: px - vb.Min.X, Y: py - vb.Min.Y}
	red := color.RGBA{R: 255, A: 255}
	gocv.Circle(&mat, center, 8, red, 2)
	gocv.Line(&mat, image.Point{X: center.X - 15, Y: center.Y}, image.Point{X: center.X + 15, Y: center.Y}, red, 2)
	gocv.Line(&mat, image.Point{X: center.X, Y: center.Y - 15}, image.Point{X: center.X, Y: center.Y + 15}, red, 2)
	out, err := mat.ToImage()
	if err != nil {
		return nil, err
	}
	return out, nil
}

// MaskImageFromFile loads a PNG (or other image) from disk for preview widgets.
func MaskImageFromFile(path string) (image.Image, error) {
	mat := gocv.IMRead(path, gocv.IMReadColor)
	if mat.Empty() {
		return nil, nil
	}
	defer mat.Close()
	img, err := mat.ToImage()
	if err != nil {
		return nil, err
	}
	return img, nil
}
