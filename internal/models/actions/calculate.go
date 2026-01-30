package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type Calculate struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Expression  string
	OutputVar   string
}

func NewCalculate(expr string, outputVar string) *Calculate {
	return &Calculate{
		BaseAction: newBaseAction("calculate"),
		Expression: expr,
		OutputVar:  outputVar,
	}
}

func (a *Calculate) String() string {
	return fmt.Sprintf("Calculate: %s -> %s", a.Expression, a.OutputVar)
}

func (a *Calculate) Icon() fyne.Resource {
	return theme.ContentAddIcon()
}
