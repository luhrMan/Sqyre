package models

import (
	"strconv"
	"strings"
)

// VariableRole describes how a macro variable is used.
type VariableRole string

const (
	VariableRoleValue   VariableRole = "value"
	VariableRoleOutput  VariableRole = "output"
	VariableRoleOutputX VariableRole = "output_x"
	VariableRoleOutputY VariableRole = "output_y"
	VariableRoleLength  VariableRole = "length"
	VariableRoleBuiltin VariableRole = "builtin"
)

// VariableType is the declared value type of a user-defined macro variable.
type VariableType string

const (
	// VariableTypeAuto stores the initial value as-is and lets runtime resolution
	// decide whether it is text or a number.
	VariableTypeAuto VariableType = "auto"
	// VariableTypeText always treats the value as a string.
	VariableTypeText VariableType = "text"
	// VariableTypeNumber parses the initial value as a number when possible.
	VariableTypeNumber VariableType = "number"
)

// VariableTypes lists every selectable type in display order.
var VariableTypes = []VariableType{VariableTypeAuto, VariableTypeText, VariableTypeNumber}

// ParseVariableType normalizes a persisted or UI string to a VariableType.
// Unknown values default to auto.
func ParseVariableType(s string) VariableType {
	switch VariableType(strings.ToLower(strings.TrimSpace(s))) {
	case VariableTypeText:
		return VariableTypeText
	case VariableTypeNumber:
		return VariableTypeNumber
	default:
		return VariableTypeAuto
	}
}

// VariableSource identifies the action that defines a variable.
type VariableSource struct {
	ActionType  string
	ActionUID   string
	ActionName  string
	Conditional bool
}

// VariableDef is a macro variable definition discovered from the action tree and declarations.
type VariableDef struct {
	Name         string
	Role         VariableRole
	Type         VariableType
	Description  string
	Source       VariableSource
	InitialValue string
}

// VariableDecl is a user-declared macro variable. It is the single persisted
// source of truth for variables the user creates and maintains in the Variables
// panel. Action-produced variables (Image Search outputs, etc.) are discovered
// separately and merged into the definition list at display time.
type VariableDecl struct {
	Name         string       `yaml:"name" mapstructure:"name"`
	Type         VariableType `yaml:"type" mapstructure:"type"`
	InitialValue string       `yaml:"initialvalue" mapstructure:"initialvalue"`
	Description  string       `yaml:"description" mapstructure:"description"`
}

// initialStoredValue coerces the declared initial value to the runtime
// representation implied by the variable's type.
func (d VariableDecl) initialStoredValue() any {
	switch d.Type {
	case VariableTypeNumber:
		trimmed := strings.TrimSpace(d.InitialValue)
		if i, err := strconv.Atoi(trimmed); err == nil {
			return i
		}
		if f, err := strconv.ParseFloat(trimmed, 64); err == nil {
			return f
		}
		return d.InitialValue
	default:
		return d.InitialValue
	}
}
