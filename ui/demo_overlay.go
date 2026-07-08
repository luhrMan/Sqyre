package ui

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"image/png"
	"math"

	"fyne.io/fyne/v2"
)

// ClickGuide marks where the user clicks in demo frames (hotspot = pointer tip).
type ClickGuide struct {
	X, Y int
}

// ClickGuideAt builds a ClickGuide from a canvas-space position.
func ClickGuideAt(pos fyne.Position) ClickGuide {
	return ClickGuide{X: int(pos.X), Y: int(pos.Y)}
}

// OverlayClickGuide draws a pointer cursor and highlight ring on a PNG screenshot.
func OverlayClickGuide(pngData []byte, guide ClickGuide) ([]byte, error) {
	src, err := png.Decode(bytes.NewReader(pngData))
	if err != nil {
		return nil, fmt.Errorf("decode png for click guide: %w", err)
	}
	bounds := src.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, src, image.Point{}, draw.Src)
	drawClickRing(out, guide.X, guide.Y)
	drawPointerCursor(out, guide.X, guide.Y)
	var buf bytes.Buffer
	if err := png.Encode(&buf, out); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// CompositePickerOverDimmedMain composites pickerPNG centered over a dimmed copy
// of mainPNG and returns the composite plus the top-left offset where the picker
// was placed (so callers can translate picker-space anchors into the composite).
func CompositePickerOverDimmedMain(mainPNG, pickerPNG []byte) ([]byte, image.Point, error) {
	base, err := png.Decode(bytes.NewReader(mainPNG))
	if err != nil {
		return nil, image.Point{}, fmt.Errorf("decode main window png: %w", err)
	}
	pickerImg, err := png.Decode(bytes.NewReader(pickerPNG))
	if err != nil {
		return nil, image.Point{}, fmt.Errorf("decode picker png: %w", err)
	}

	bounds := base.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, base, image.Point{}, draw.Src)
	scrim := image.NewUniform(color.NRGBA{R: 0, G: 0, B: 0, A: 160})
	draw.Draw(out, bounds, scrim, image.Point{}, draw.Over)

	dx := (bounds.Dx() - pickerImg.Bounds().Dx()) / 2
	dy := (bounds.Dy() - pickerImg.Bounds().Dy()) / 2
	offset := image.Pt(dx, dy)
	pickerRect := pickerImg.Bounds().Add(offset)
	draw.Draw(out, pickerRect, pickerImg, pickerImg.Bounds().Min, draw.Over)

	var buf bytes.Buffer
	if err := png.Encode(&buf, out); err != nil {
		return nil, image.Point{}, err
	}
	return buf.Bytes(), offset, nil
}

func drawClickRing(img *image.RGBA, cx, cy int) {
	ring := color.NRGBA{R: 255, G: 160, B: 40, A: 220}
	inner := color.NRGBA{R: 255, G: 160, B: 40, A: 60}
	for angle := 0.0; angle < 360; angle += 0.5 {
		rad := angle * math.Pi / 180
		x := cx + int(14*math.Cos(rad))
		y := cy + int(14*math.Sin(rad))
		setPixel(img, x, y, ring)
		setPixel(img, x+1, y, ring)
	}
	for angle := 0.0; angle < 360; angle += 2 {
		rad := angle * math.Pi / 180
		x := cx + int(8*math.Cos(rad))
		y := cy + int(8*math.Sin(rad))
		setPixel(img, x, y, inner)
	}
}

func drawPointerCursor(img *image.RGBA, tipX, tipY int) {
	// Classic arrow cursor; (tipX, tipY) is the hotspot (top-left of arrow).
	outline := color.NRGBA{R: 0, G: 0, B: 0, A: 255}
	fill := color.NRGBA{R: 255, G: 255, B: 255, A: 255}
	pts := []image.Point{
		{0, 0},
		{0, 17},
		{4, 13},
		{7, 20},
		{10, 19},
		{7, 12},
		{12, 12},
	}
	for _, p := range pts {
		p.X += tipX
		p.Y += tipY
	}
	for y := -1; y <= 1; y++ {
		for x := -1; x <= 1; x++ {
			if x == 0 && y == 0 {
				continue
			}
			offset := []image.Point{{x, y}}
			drawPolygon(img, offsetPts(pts, offset[0]), outline)
		}
	}
	drawPolygon(img, pts, fill)
	drawPolygon(img, pts, outline)
}

func offsetPts(pts []image.Point, d image.Point) []image.Point {
	out := make([]image.Point, len(pts))
	for i, p := range pts {
		out[i] = p.Add(d)
	}
	return out
}

func drawPolygon(img *image.RGBA, pts []image.Point, c color.Color) {
	if len(pts) < 3 {
		return
	}
	minY, maxY := pts[0].Y, pts[0].Y
	for _, p := range pts[1:] {
		if p.Y < minY {
			minY = p.Y
		}
		if p.Y > maxY {
			maxY = p.Y
		}
	}
	for y := minY; y <= maxY; y++ {
		var crossings []float64
		for i := range pts {
			j := (i + 1) % len(pts)
			yi, yj := float64(pts[i].Y), float64(pts[j].Y)
			if yi == yj {
				continue
			}
			if (y >= int(math.Min(yi, yj))) && (y < int(math.Max(yi, yj))) {
				xi, xj := float64(pts[i].X), float64(pts[j].X)
				x := xi + (float64(y)-yi)*(xj-xi)/(yj-yi)
				crossings = append(crossings, x)
			}
		}
		for i := 0; i < len(crossings); i++ {
			for j := i + 1; j < len(crossings); j++ {
				if crossings[j] < crossings[i] {
					crossings[i], crossings[j] = crossings[j], crossings[i]
				}
			}
		}
		for i := 0; i+1 < len(crossings); i += 2 {
			for x := int(math.Ceil(crossings[i])); x < int(math.Floor(crossings[i+1])); x++ {
				setPixel(img, x, y, c)
			}
		}
	}
}

func setPixel(img *image.RGBA, x, y int, c color.Color) {
	if !(image.Point{x, y}.In(img.Bounds())) {
		return
	}
	img.Set(x, y, c)
}
