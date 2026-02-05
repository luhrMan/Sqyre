package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
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
	return fmt.Sprintf("Set %s = %v", a.VariableName, a.Value)
}

func (a *SetVariable) Icon() fyne.Resource {
	return theme.DocumentIcon()
}
