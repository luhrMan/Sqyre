package actions

type advancedAction struct {
        baseAction                   //`json:"baseaction"`
        Name       string            `json:"name"`
        SubActions []ActionInterface `json:"subactions"`
}

func newAdvancedAction(name string, subActions []ActionInterface) *advancedAction {
        return &advancedAction{
                baseAction: newBaseAction(),
                Name:       name,
                SubActions: subActions,
        }
}
