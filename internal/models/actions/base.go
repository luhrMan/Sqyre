package actions

import "github.com/google/uuid"

type BaseAction struct {
	Type   string
	uid    string
	Parent AdvancedActionInterface `yaml:"-" gob:"-" mapstructure:"-"`
}

func (a *BaseAction) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.Type),
	}
}

func newBaseAction(t string) *BaseAction {
	return &BaseAction{
		Type: t,
		uid:  uuid.NewString(),
	}
}
