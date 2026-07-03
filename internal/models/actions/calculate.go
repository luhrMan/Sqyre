package actions

type Calculate struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Expression  string
	OutputVar   string
}

func NewCalculate(expr string, outputVar string) *Calculate {
	return &Calculate{
		BaseAction: newBaseAction("calculate"),
		Expression: expr,
		OutputVar:  outputVar,
	}
}

func (a *Calculate) String() string {
	return stringifyParams(a.Params())
}

func (a *Calculate) Params() []Param {
	return []Param{
		newParam("Type", a.GetType()),
		newParam("Expression", a.Expression),
		newParam("Output", a.OutputVar),
	}
}

func (a *Calculate) VariableBindings() []VariableBinding {
	if a.OutputVar == "" {
		return nil
	}
	return []VariableBinding{{Name: a.OutputVar, Role: "output"}}
}
