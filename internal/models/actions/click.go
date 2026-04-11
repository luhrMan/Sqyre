package actions

type Click struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Button      bool `yaml:"button" mapstructure:"button"`
	State       bool `yaml:"state" mapstructure:"state"`
}

func NewClick(button bool, state bool) *Click {
	return &Click{
		BaseAction: newBaseAction("click"),
		Button:     button,
		State:      state,
	}
}

func (a *Click) String() string           { return stringifyParams(a.parameters()) }
func (a *Click) Parameters() []ActionParam { return a.parameters() }

func (a *Click) parameters() []ActionParam {
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Button", LeftOrRight(a.Button)),
		newParam("State", UpOrDown(a.State)),
	}
}

func LeftOrRight(b bool) string {
	if b {
		return "right"
	}
	return "left"
}
