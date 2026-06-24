package actions

type AdvancedAction struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Name        string
	SubActions  []ActionInterface `yaml:",omitempty" mapstructure:",omitempty"`
}

func (a *AdvancedAction) parameters() []actionParam {
	return []actionParam{
		newParam("Name", a.Name),
		newParam("Type", a.Type),
	}
}

func newAdvancedAction(name, t string, subActions []ActionInterface) *AdvancedAction {
	return &AdvancedAction{
		BaseAction: newBaseAction(t),
		Name:       name,
		SubActions: subActions,
	}
}
