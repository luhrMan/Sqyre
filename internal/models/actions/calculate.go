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

func (a *Calculate) String() string           { return stringifyParams(a.parameters()) }
func (a *Calculate) Parameters() []ActionParam { return a.parameters() }

func (a *Calculate) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Expression", a.Expression),
		newParam("Output", a.OutputVar),
	}
}
