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

// SearchArea represents a rectangular region for image search and ocr operations.
// LeftX, TopY, RightX, and BottomY may be int (literal) or string (variable reference e.g. "${leftX}").
type SearchArea struct {
	Name    string
	LeftX   interface{}
	TopY    interface{}
	RightX  interface{}
	BottomY interface{}
}
