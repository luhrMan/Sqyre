package vision

import (
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"errors"
	"fmt"
	"image"
)

// PointPreviewTooltip captures a downscaled point preview for hover tooltips.
func PointPreviewTooltip(point *models.Point) (image.Image, error) {
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
	return CapturePointPreviewTooltip(px, py)
}

// SearchAreaPreviewTooltip captures a downscaled search-area preview for hover tooltips.
func SearchAreaPreviewTooltip(searchArea *models.SearchArea) (image.Image, error) {
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
	img, err := CaptureSearchAreaPreviewTooltip(lx, ty, rx, by)
	if err != nil {
		return nil, fmt.Errorf("SearchArea: %w (area: %s)", err, searchArea.Name)
	}
	return img, nil
}

// PointPreviewTooltipCached returns a cached tooltip preview for point when available.
func PointPreviewTooltipCached(point *models.Point) (image.Image, string, error) {
	if point == nil {
		return nil, "", errors.New("Point: Cannot update preview - point is nil")
	}
	key := previewCacheKeyPoint(point)
	if img, caption, ok := getPreviewTooltipCached(key); ok {
		return img, caption, nil
	}
	caption := PointPreviewCaption(point)
	img, err := PointPreviewTooltip(point)
	if err != nil {
		return nil, "", err
	}
	putPreviewTooltipCached(key, img, caption)
	return cloneImage(img), caption, nil
}

// SearchAreaPreviewTooltipCached returns a cached tooltip preview for searchArea when available.
func SearchAreaPreviewTooltipCached(searchArea *models.SearchArea) (image.Image, string, error) {
	if searchArea == nil {
		return nil, "", errors.New("SearchArea: Cannot update preview - search area is nil")
	}
	key := previewCacheKeySearchArea(searchArea)
	if img, caption, ok := getPreviewTooltipCached(key); ok {
		return img, caption, nil
	}
	caption := SearchAreaPreviewCaption(searchArea)
	img, err := SearchAreaPreviewTooltip(searchArea)
	if err != nil {
		return nil, "", err
	}
	putPreviewTooltipCached(key, img, caption)
	return cloneImage(img), caption, nil
}
