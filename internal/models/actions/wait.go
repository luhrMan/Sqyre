package actions

import (
	"fmt"

)

type Wait struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	// Time is the wait duration in milliseconds: int (literal) or string (variable reference e.g. "${delay}").
	Time any
}

func NewWait(time any) *Wait {
	if time == nil {
		time = 0
	}
	return &Wait{
		BaseAction: newBaseAction("wait"),
		Time:       time,
	}
}

func (a *Wait) String() string {
	return stringifyParams(a.Params())
}

func formatWaitTime(t any) string {
	switch v := t.(type) {
	case int:
		return fmt.Sprintf("%d ms", v)
	case float64:
		return fmt.Sprintf("%.0f ms", v)
	case string:
		return v
	default:
		if t == nil {
			return "0 ms"
		}
		return fmt.Sprintf("%v", t)
	}
}

func (a *Wait) Params() []Param {
	return []Param{
		newParam("Type", a.GetType()),
		newParam("Time", formatWaitTime(a.Time)),
	}
}

