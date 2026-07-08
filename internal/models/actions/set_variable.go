package actions

type SetVariable struct {
	*BaseAction  `yaml:",inline" mapstructure:",squash"`
	VariableName string
	Value        any // Can be string, int, float, etc.
}

func NewSetVariable(name string, value any) *SetVariable {
	return &SetVariable{
		BaseAction:   newBaseAction("setvariable"),
		VariableName: name,
		Value:        value,
	}
}

func (a *SetVariable) String() string {
	return stringifyParams(a.Params())
}

func (a *SetVariable) Params() []Param {
	return []Param{
		newParam("Type", a.GetType()),
		newParam("Variable", a.VariableName),
		newParam("Value", a.Value),
	}
}

func (a *SetVariable) VariableBindings() []VariableBinding {
	if a.VariableName == "" {
		return nil
	}
	return []VariableBinding{{Name: a.VariableName, Role: "value"}}
}
