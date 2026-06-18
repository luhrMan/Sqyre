package screen

import (
	"fmt"
	"image"
)

const maxSearchAreaPixels int64 = 64 * 1024 * 1024

// EnabledDisplayRects returns absolute bounds for each enabled display.
func EnabledDisplayRects() []image.Rectangle {
	n := NumDisplays()
	enabled := EnabledMonitorIndices()
	var indices []int
	if enabled == nil {
		for i := 0; i < n; i++ {
			indices = append(indices, i)
		}
	} else {
		indices = enabled
	}
	rects := make([]image.Rectangle, 0, len(indices))
	for _, i := range indices {
		b := DisplayBoundsAbs(i)
		if !b.Empty() {
			rects = append(rects, b)
		}
	}
	if len(rects) == 0 {
		if vb := virtualBoundsImpl(); !vb.Empty() {
			rects = append(rects, vb)
		}
	}
	return rects
}

// ValidateSearchAreaRect normalizes the search rectangle, crops edges that extend past
// enabled displays, and checks that the result has positive size and sane limits.
func ValidateSearchAreaRect(leftX, topY, rightX, bottomY int) (lx, ty, rx, by, w, h int, err error) {
	return validateSearchAreaOnDisplays(leftX, topY, rightX, bottomY, EnabledDisplayRects())
}

func validateSearchAreaOnDisplays(leftX, topY, rightX, bottomY int, displays []image.Rectangle) (int, int, int, int, int, int, error) {
	if leftX > rightX {
		leftX, rightX = rightX, leftX
	}
	if topY > bottomY {
		topY, bottomY = bottomY, topY
	}
	leftX, topY, rightX, bottomY = fitSearchAreaToDisplays(leftX, topY, rightX, bottomY, displays)
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		return leftX, topY, rightX, bottomY, w, h, fmt.Errorf("invalid search area (width=%d height=%d); need positive dimensions", w, h)
	}
	if w > 1<<16 || h > 1<<16 {
		return leftX, topY, rightX, bottomY, w, h, fmt.Errorf("search area dimensions too large (%dx%d)", w, h)
	}
	area := int64(w) * int64(h)
	if area > maxSearchAreaPixels {
		return leftX, topY, rightX, bottomY, w, h, fmt.Errorf("search area too large (%d pixels)", area)
	}
	r := image.Rect(leftX, topY, rightX, bottomY)
	if !rectFullyOnDisplays(r, displays) {
		return leftX, topY, rightX, bottomY, w, h, searchAreaCoverageError(r, displays)
	}
	return leftX, topY, rightX, bottomY, w, h, nil
}

// fitSearchAreaToDisplays crops sides that extend past enabled displays. When the area
// still spans a monitor gap, it keeps the largest single-monitor intersection.
func fitSearchAreaToDisplays(leftX, topY, rightX, bottomY int, displays []image.Rectangle) (int, int, int, int) {
	r := image.Rect(leftX, topY, rightX, bottomY)
	if rectFullyOnDisplays(r, displays) {
		return leftX, topY, rightX, bottomY
	}
	if u, ok := unionDisplayBounds(displays); ok {
		leftX, topY, rightX, bottomY = cropSearchAreaToUnion(leftX, topY, rightX, bottomY, u)
		r = image.Rect(leftX, topY, rightX, bottomY)
		if rectFullyOnDisplays(r, displays) {
			return leftX, topY, rightX, bottomY
		}
	}
	if best, ok := largestDisplayIntersection(r, displays); ok {
		return best.Min.X, best.Min.Y, best.Max.X, best.Max.Y
	}
	return leftX, topY, rightX, bottomY
}

func unionDisplayBounds(displays []image.Rectangle) (image.Rectangle, bool) {
	if len(displays) == 0 {
		return image.Rectangle{}, false
	}
	u := displays[0]
	for _, d := range displays[1:] {
		u = u.Union(d)
	}
	return u, !u.Empty()
}

func cropSearchAreaToUnion(leftX, topY, rightX, bottomY int, u image.Rectangle) (int, int, int, int) {
	if leftX < u.Min.X {
		leftX = u.Min.X
	}
	if topY < u.Min.Y {
		topY = u.Min.Y
	}
	if rightX > u.Max.X {
		rightX = u.Max.X
	}
	if bottomY > u.Max.Y {
		bottomY = u.Max.Y
	}
	return leftX, topY, rightX, bottomY
}

func largestDisplayIntersection(r image.Rectangle, displays []image.Rectangle) (image.Rectangle, bool) {
	var best image.Rectangle
	var bestArea int64
	for _, d := range displays {
		inter := r.Intersect(d)
		if inter.Empty() {
			continue
		}
		area := int64(inter.Dx()) * int64(inter.Dy())
		if area > bestArea {
			bestArea = area
			best = inter
		}
	}
	return best, bestArea > 0
}

func rectFullyOnDisplays(r image.Rectangle, displays []image.Rectangle) bool {
	if r.Empty() || len(displays) == 0 {
		return false
	}
	need := int64(r.Dx()) * int64(r.Dy())
	if need <= 0 {
		return false
	}
	var covered int64
	for _, d := range displays {
		inter := r.Intersect(d)
		if !inter.Empty() {
			covered += int64(inter.Dx()) * int64(inter.Dy())
		}
	}
	return covered == need
}

func searchAreaCoverageError(r image.Rectangle, displays []image.Rectangle) error {
	if len(displays) == 0 {
		return fmt.Errorf("search area (%d,%d) to (%d,%d) is not on any display", r.Min.X, r.Min.Y, r.Max.X, r.Max.Y)
	}
	u := displays[0]
	for _, d := range displays[1:] {
		u = u.Union(d)
	}
	return fmt.Errorf(
		"search area (%d,%d) to (%d,%d) is not fully on a display (enabled displays span (%d,%d)..(%d,%d) with possible gaps between monitors)",
		r.Min.X, r.Min.Y, r.Max.X, r.Max.Y, u.Min.X, u.Min.Y, u.Max.X, u.Max.Y,
	)
}
