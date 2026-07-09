package validation

import (
	"fmt"
	"strings"
)

// ValidateCollectionFields checks collection rows/cols and linked search area name.
func ValidateCollectionFields(searchArea, rows, cols string) error {
	if strings.TrimSpace(searchArea) == "" {
		return fmt.Errorf("search area is required")
	}
	if _, err := ParsePositiveInt(rows); err != nil {
		return fmt.Errorf("rows: %w", err)
	}
	if _, err := ParsePositiveInt(cols); err != nil {
		return fmt.Errorf("cols: %w", err)
	}
	return nil
}
