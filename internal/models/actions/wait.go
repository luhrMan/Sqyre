package actions

import "fmt"

type Wait struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Time        int
}

func NewWait(time int) *Wait {
	return &Wait{
		BaseAction: newBaseAction("wait"),
		Time:       time,
	}
}

func (a *Wait) String() string           { return stringifyParams(a.parameters()) }
func (a *Wait) Parameters() []ActionParam { return a.parameters() }

func (a *Wait) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Time", fmt.Sprintf("%d ms", a.Time)),
	}
}
