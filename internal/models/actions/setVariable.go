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

func (a *SetVariable) String() string           { return stringifyParams(a.parameters()) }
func (a *SetVariable) Parameters() []ActionParam { return a.parameters() }

func (a *SetVariable) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Variable", a.VariableName),
		newParam("Value", a.Value),
	}
}
