package actions

// VariableBinding describes a macro variable name produced by an action.
type VariableBinding struct {
	Name        string
	Role        string // value, output, output_x, output_y, length
	Conditional bool   // true when the variable is only set in some execution paths
}

// VariableProducer is implemented by actions that define or write macro variables.
type VariableProducer interface {
	VariableBindings() []VariableBinding
}
