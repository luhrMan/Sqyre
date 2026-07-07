package validation

import (
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"fmt"
)

// intCoord returns an int when v is a numeric coordinate literal.
func intCoord(v any) (int, bool) {
	switch n := v.(type) {
	case int:
		return n, true
	case float64:
		return int(n), true
	default:
		return 0, false
	}
}

// ValidateSearchAreaLiteralBounds validates geometry when all coordinates are numeric literals.
// Variable references skip bounds checks because values are unknown at edit time.
func ValidateSearchAreaLiteralBounds(leftX, topY, rightX, bottomY any) error {
	lx, okLX := intCoord(leftX)
	ty, okTY := intCoord(topY)
	rx, okRX := intCoord(rightX)
	by, okBY := intCoord(bottomY)
	if !okLX || !okTY || !okRX || !okBY {
		return nil
	}
	_, _, _, _, _, _, err := screen.ValidateSearchAreaRect(lx, ty, rx, by)
	if err != nil {
		return err
	}
	return nil
}

// ValidateSearchAreaSave checks a search area before persisting it.
func ValidateSearchAreaSave(sa *models.SearchArea) error {
	if sa == nil {
		return fmt.Errorf("search area cannot be nil")
	}
	if err := ValidateEntityName(sa.Name); err != nil {
		return err
	}
	return ValidateSearchAreaLiteralBounds(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY)
}
