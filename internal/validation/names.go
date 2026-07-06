package validation

import (
	"fmt"
	"strings"
	"unicode"
)

// ValidateEntityName checks a program, item, point, or other named entity label.
func ValidateEntityName(name string) error {
	if strings.TrimSpace(name) == "" {
		return ErrEmptyName
	}
	return nil
}

// ValidateVariableName checks a macro variable identifier used in ${name} references.
func ValidateVariableName(name string) error {
	name = strings.TrimSpace(name)
	if name == "" {
		return ErrEmptyName
	}
	if strings.ContainsAny(name, "${}") {
		return fmt.Errorf("%w: must not contain $, {, or }", ErrInvalidVariable)
	}
	for _, r := range name {
		if unicode.IsControl(r) {
			return fmt.Errorf("%w: must not contain control characters", ErrInvalidVariable)
		}
	}
	return nil
}
