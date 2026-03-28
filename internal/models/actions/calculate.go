package actions

import (
	"Sqyre/internal/assets"

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
	return stringifyParams(a.parameters())
}

func (a *Calculate) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Calculate) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Expression", a.Expression),
		newParam("Output", a.OutputVar),
	}
}

func (a *Calculate) Icon() fyne.Resource {
	return assets.CalculateIcon
}
