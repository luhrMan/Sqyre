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

// SearchArea represents a rectangular region for image search and ocr operations
type SearchArea struct {
	Name    string
	LeftX   int
	TopY    int
	RightX  int
	BottomY int
}
