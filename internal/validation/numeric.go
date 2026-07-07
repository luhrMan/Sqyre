package validation

import (
	"fmt"
	"strconv"
	"strings"
)

// ParsePositiveInt parses s as an integer that must be greater than zero.
func ParsePositiveInt(s string) (int, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, ErrNotPositiveInt
	}
	v, err := strconv.Atoi(s)
	if err != nil || v <= 0 {
		return 0, fmt.Errorf("%w: %q", ErrNotPositiveInt, s)
	}
	return v, nil
}

// ParseNonNegativeInt parses s as an integer that must be zero or greater.
func ParseNonNegativeInt(s string) (int, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, ErrNegativeInt
	}
	v, err := strconv.Atoi(s)
	if err != nil || v < 0 {
		return 0, fmt.Errorf("%w: %q", ErrNegativeInt, s)
	}
	return v, nil
}

// ValidateItemGridFields checks item grid dimensions and stack limit text fields.
func ValidateItemGridFields(cols, rows, stackMax string) error {
	if _, err := ParsePositiveInt(cols); err != nil {
		return fmt.Errorf("cols: %w", err)
	}
	if _, err := ParsePositiveInt(rows); err != nil {
		return fmt.Errorf("rows: %w", err)
	}
	if _, err := ParseNonNegativeInt(stackMax); err != nil {
		return fmt.Errorf("stack max: %w", err)
	}
	return nil
}
