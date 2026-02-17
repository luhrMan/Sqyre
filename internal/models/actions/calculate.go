package actions

import (
	"Squire/internal/assets"
	"fmt"

	"fyne.io/fyne/v2"
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
	return assets.CalculateIcon
}
