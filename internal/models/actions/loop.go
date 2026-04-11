package actions

type Loop struct {
	// Count is the number of iterations: int (literal) or string (variable reference e.g. "${count}").
	Count           any
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewLoop(count any, name string, subActions []ActionInterface) *Loop {
	if name == "root" {
		r := &Loop{
			AdvancedAction: newAdvancedAction(name, "loop", subActions),
			Count:          1,
		}
		r.uid = ""
		r.SetParent(nil)
		return r
	}
	if count == nil {
		count = 1
	}
	return &Loop{
		AdvancedAction: newAdvancedAction(name, "loop", subActions),
		Count:          count,
	}
}

func (a *Loop) String() string           { return stringifyParams(a.parameters()) }
func (a *Loop) Parameters() []ActionParam { return a.parameters() }

func (a *Loop) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Iterations", a.Count),
	}
}
