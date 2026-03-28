package actions

import (
	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

type SetVariable struct {
	*BaseAction  `yaml:",inline" mapstructure:",squash"`
	VariableName string
	Value        any // Can be string, int, float, etc.
}

func NewSetVariable(name string, value any) *SetVariable {
	return &SetVariable{
		BaseAction:   newBaseAction("setvariable"),
		VariableName: name,
		Value:        value,
	}
}

func (a *SetVariable) String() string {
	return stringifyParams(a.parameters())
}

func (a *SetVariable) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *SetVariable) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Variable", a.VariableName),
		newParam("Value", a.Value),
	}
}

func (a *SetVariable) Icon() fyne.Resource {
	return assets.VariableIcon
}
