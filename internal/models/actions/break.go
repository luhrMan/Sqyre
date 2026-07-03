package actions

// Break exits the innermost enclosing loop (Loop, ForEachRow, or ImageSearch
// per-match iteration).
type Break struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
}

func NewBreak() *Break {
	return &Break{
		BaseAction: newBaseAction("break"),
	}
}

func (a *Break) String() string {
	return stringifyParams(a.Params())
}

func (a *Break) Params() []Param {
	return []Param{
		newParam("Type", a.GetType()),
	}
}

