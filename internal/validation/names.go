package validation

import (
	"Sqyre/internal/macro"
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

// ValidateVariableAssignmentName checks a variable name used when setting/defining a macro variable.
// Expressions and reference syntax are rejected.
func ValidateVariableAssignmentName(name string) error {
	if err := ValidateVariableName(name); err != nil {
		return err
	}
	if macro.LooksLikeExpression(name) {
		return fmt.Errorf("%w: must be a simple variable name, not an expression", ErrInvalidVariable)
	}
	return nil
}
