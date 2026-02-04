package actions

import (
	"fmt"

	"Squire/internal/assets"

	"fyne.io/fyne/v2"
)

type Ocr struct {
	Target          string
	SearchArea      SearchArea
	OutputVariable  string
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox SearchArea) *Ocr {
	return &Ocr{
		AdvancedAction: newAdvancedAction(name, "ocr", subActions),
		Target:         target,
		SearchArea:     searchbox,
		OutputVariable: "",
	}
}

func (a *Ocr) String() string {
	return fmt.Sprintf("%s | `%s` in `%s`", a.Name, a.Target, a.SearchArea.Name)
}

func (a *Ocr) Icon() fyne.Resource {
	return assets.TextSearchIcon
}
