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
	// Preprocessing: Blur 0-30 (0=off), MinThreshold 0-255 (0=off), Resize 1.0-10.0, Grayscale
	Blur         int
	MinThreshold int
	Resize       float64
	Grayscale    bool
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox SearchArea) *Ocr {
	return &Ocr{
		AdvancedAction: newAdvancedAction(name, "ocr", subActions),
		Target:         target,
		SearchArea:     searchbox,
		OutputVariable: "",
		Blur:           3,
		MinThreshold:   50,
		Resize:         1.0,
		Grayscale:      true,
	}
}

func (a *Ocr) String() string {
	return fmt.Sprintf("%s | `%s` in `%s`", a.Name, a.Target, a.SearchArea.Name)
}

func (a *Ocr) Icon() fyne.Resource {
	return assets.TextSearchIcon
}
