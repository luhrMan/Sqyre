package services

import (
	"Sqyre/internal/capture"
	"Sqyre/internal/vision"
	"image"
	"strconv"

	"gocv.io/x/gocv"
)

// findPixelInRGBA locates the first pixel matching target RGB within tolerance.
// Returns coordinates relative to rgba.Bounds().Min.
func findPixelInRGBA(rgba *image.RGBA, tr, tg, tb uint8, tolerance int) (px, py int, ok bool) {
	if rgba == nil {
		return 0, 0, false
	}
	bounds := rgba.Bounds()
	if bounds.Empty() {
		return 0, 0, false
	}
	if tolerance >= 100 {
		return bounds.Min.X, bounds.Min.Y, true
	}

	var found bool
	vision.WithOpenCV(func() {
		mat, err := gocv.ImageToMatRGB(rgba)
		if err != nil || mat.Empty() {
			return
		}
		defer mat.Close()

		if tolerance <= 0 {
			scalar := gocv.NewScalar(float64(tb), float64(tg), float64(tr), 0)
			lower := gocv.NewMatWithSize(1, 1, gocv.MatTypeCV8UC3)
			defer lower.Close()
			upper := gocv.NewMatWithSize(1, 1, gocv.MatTypeCV8UC3)
			defer upper.Close()
			lower.SetTo(scalar)
			upper.SetTo(scalar)
			mask := gocv.NewMat()
			defer mask.Close()
			if gocv.InRange(mat, lower, upper, &mask) != nil {
				return
			}
			px, py, found = firstPointInMask(mask, bounds.Min.X, bounds.Min.Y)
			return
		}

		delta := uint8(255 * tolerance / 100)
		lower := gocv.NewMatWithSize(1, 1, gocv.MatTypeCV8UC3)
		defer lower.Close()
		upper := gocv.NewMatWithSize(1, 1, gocv.MatTypeCV8UC3)
		defer upper.Close()
		lower.SetTo(gocv.NewScalar(float64(clampU8(tb, delta, false)), float64(clampU8(tg, delta, false)), float64(clampU8(tr, delta, false)), 0))
		upper.SetTo(gocv.NewScalar(float64(clampU8(tb, delta, true)), float64(clampU8(tg, delta, true)), float64(clampU8(tr, delta, true)), 0))
		mask := gocv.NewMat()
		defer mask.Close()
		if gocv.InRange(mat, lower, upper, &mask) != nil {
			return
		}
		px, py, found = firstPointInMask(mask, bounds.Min.X, bounds.Min.Y)
	})
	return px, py, found
}

func firstPointInMask(mask gocv.Mat, offsetX, offsetY int) (int, int, bool) {
	idx := gocv.NewMat()
	defer idx.Close()
	if gocv.FindNonZero(mask, &idx) != nil || idx.Empty() || idx.Rows() == 0 {
		return 0, 0, false
	}
	x := idx.GetIntAt(0, 0)
	y := idx.GetIntAt(0, 1)
	return offsetX + int(x), offsetY + int(y), true
}

func clampU8(channel, delta uint8, upper bool) uint8 {
	if upper {
		if int(channel)+int(delta) > 255 {
			return 255
		}
		return channel + delta
	}
	if int(channel) < int(delta) {
		return 0
	}
	return channel - delta
}

// findPixelInCapture scans a captured image for a matching pixel and returns screen coords.
func findPixelInCapture(captureImg image.Image, capLeftX, capTopY int, tr, tg, tb uint8, tolerance int) (screenX, screenY int, ok bool) {
	rgba := capture.CaptureToRGBA(captureImg)
	px, py, found := findPixelInRGBA(rgba, tr, tg, tb, tolerance)
	if !found {
		return 0, 0, false
	}
	bounds := rgba.Bounds()
	return capLeftX + px - bounds.Min.X, capTopY + py - bounds.Min.Y, true
}

// rgbFromHex parses a normalized 6-char hex color into RGB channels.
func rgbFromHex(hex string) (r, g, b uint8, ok bool) {
	if len(hex) != 6 {
		return 0, 0, 0, false
	}
	rr, err1 := strconv.ParseUint(hex[0:2], 16, 8)
	gg, err2 := strconv.ParseUint(hex[2:4], 16, 8)
	bb, err3 := strconv.ParseUint(hex[4:6], 16, 8)
	if err1 != nil || err2 != nil || err3 != nil {
		return 0, 0, 0, false
	}
	return uint8(rr), uint8(gg), uint8(bb), true
}
