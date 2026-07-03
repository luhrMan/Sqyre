package vision

import (
	"Sqyre/internal/config"
	"Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/screen"
	"errors"
	"fmt"
	"image"
)

func coordToIntForPreview(v any) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

// PointPreview captures a cropped screen preview around point with a crosshair overlay.
func PointPreview(point *models.Point) (image.Image, error) {
	if point == nil {
		return nil, errors.New("Point: Cannot update preview - point is nil")
	}
	px := coordToIntForPreview(point.X)
	py := coordToIntForPreview(point.Y)
	vb := screen.VirtualBounds()
	if px < vb.Min.X || py < vb.Min.Y || px > vb.Max.X || py > vb.Max.Y {
		return nil, fmt.Errorf("Point: Point outside virtual desktop - desktop: (%d,%d)..(%d,%d), point: (%d,%d) (point: %s)",
			vb.Min.X, vb.Min.Y, vb.Max.X, vb.Max.Y, px, py, point.Name)
	}
	return CapturePointPreview(px, py)
}

// SearchAreaPreview captures a cropped screen preview around searchArea with a rectangle overlay.
func SearchAreaPreview(searchArea *models.SearchArea) (image.Image, error) {
	if searchArea == nil {
		return nil, errors.New("SearchArea: Cannot update preview - search area is nil")
	}
	lx := coordToIntForPreview(searchArea.LeftX)
	ty := coordToIntForPreview(searchArea.TopY)
	rx := coordToIntForPreview(searchArea.RightX)
	by := coordToIntForPreview(searchArea.BottomY)
	lx, ty, rx, by, _, _, err := screen.ValidateSearchAreaRect(lx, ty, rx, by)
	if err != nil {
		return nil, fmt.Errorf("SearchArea: %w (area: %s)", err, searchArea.Name)
	}
	img, err := CaptureSearchAreaPreview(lx, ty, rx, by)
	if err != nil {
		return nil, fmt.Errorf("SearchArea: %w (area: %s)", err, searchArea.Name)
	}
	return img, nil
}

// PointPreviewCaption formats point coordinates for preview tooltips.
func PointPreviewCaption(point *models.Point) string {
	if point == nil {
		return ""
	}
	return fmt.Sprintf("X: %v, Y: %v", point.X, point.Y)
}

// SearchAreaPreviewCaption formats search area coordinates for preview tooltips.
func SearchAreaPreviewCaption(searchArea *models.SearchArea) string {
	if searchArea == nil {
		return ""
	}
	return fmt.Sprintf("Left: %v, Top: %v, Right: %v, Bottom: %v", searchArea.LeftX, searchArea.TopY, searchArea.RightX, searchArea.BottomY)
}

// PointPreviewTooltipForRef captures a point preview and caption from a macro CoordinateRef.
func PointPreviewTooltipForRef(ref actions.CoordinateRef) (image.Image, string, error) {
	pt, err := macro.LookupPoint(ref, config.MainMonitorSizeString)
	if err != nil {
		return nil, "", err
	}
	img, err := PointPreview(pt)
	return img, PointPreviewCaption(pt), err
}

// SearchAreaPreviewTooltipForRef captures a search area preview and caption from a macro CoordinateRef.
func SearchAreaPreviewTooltipForRef(ref actions.CoordinateRef) (image.Image, string, error) {
	sa, err := macro.LookupSearchArea(ref, config.MainMonitorSizeString)
	if err != nil {
		return nil, "", err
	}
	img, err := SearchAreaPreview(sa)
	return img, SearchAreaPreviewCaption(sa), err
}

// PointPreviewForRef captures a point preview from a macro CoordinateRef.
func PointPreviewForRef(ref actions.CoordinateRef) (image.Image, error) {
	pt, err := macro.LookupPoint(ref, config.MainMonitorSizeString)
	if err != nil {
		return nil, err
	}
	return PointPreview(pt)
}

// SearchAreaPreviewForRef captures a search area preview from a macro CoordinateRef.
func SearchAreaPreviewForRef(ref actions.CoordinateRef) (image.Image, error) {
	sa, err := macro.LookupSearchArea(ref, config.MainMonitorSizeString)
	if err != nil {
		return nil, err
	}
	return SearchAreaPreview(sa)
}
