package actions

type AdvancedAction struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Name        string
	SubActions  []ActionInterface `yaml:",omitempty" mapstructure:",omitempty"`
}

func newAdvancedAction(name, t string, subActions []ActionInterface) *AdvancedAction {
	return &AdvancedAction{
		BaseAction: newBaseAction(t),
		Name:       name,
		SubActions: subActions,
	}
}
