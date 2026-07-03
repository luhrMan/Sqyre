package actions

// Continue skips the rest of the current loop iteration and advances to the next
// iteration of the innermost enclosing loop.
type Continue struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
}

func NewContinue() *Continue {
	return &Continue{
		BaseAction: newBaseAction("continue"),
	}
}

func (a *Continue) String() string {
	return stringifyParams(a.Params())
}

func (a *Continue) Params() []Param {
	return []Param{
		newParam("Type", a.GetType()),
	}
}

