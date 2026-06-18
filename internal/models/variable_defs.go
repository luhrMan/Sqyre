package models

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

// VariableSource identifies the action that defines a variable.
type VariableSource struct {
	ActionType  string
	ActionUID   string
	ActionName  string
	Conditional bool
}

// VariableDef is a macro variable definition discovered from the action tree and store.
type VariableDef struct {
	Name         string
	Role         VariableRole
	Source       VariableSource
	InitialValue string
}
