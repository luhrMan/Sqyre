package actions

import (
	"fmt"
	"slices"

	"Squire/internal/assets"

	"fyne.io/fyne/v2"
)

type ImageSearch struct {
	Targets         []string   `mapstructure:"targets"`
	SearchArea      SearchArea `mapstructure:"searcharea"`
	RowSplit        int        `mapstructure:"rowsplit"`
	ColSplit        int        `mapstructure:"colsplit"`
	Tolerance       float32    `mapstructure:"tolerance"`
	Blur            int        `mapstructure:"blur"`
	OutputXVariable string     `mapstructure:"outputxvariable"` // Variable name to store X coordinate
	OutputYVariable string     `mapstructure:"outputyvariable"` // Variable name to store Y coordinate
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewImageSearch(name string, subActions []ActionInterface, targets []string, searchbox SearchArea, rs, cs int, tol float32, blur int) *ImageSearch {
	slices.Sort(targets)
	return &ImageSearch{
		AdvancedAction:  newAdvancedAction(name, "imagesearch", subActions),
		Targets:         targets,
		SearchArea:      searchbox,
		RowSplit:        rs,
		ColSplit:        cs,
		Tolerance:       tol,
		Blur:            blur,
		OutputXVariable: "",
		OutputYVariable: "",
	}
}
func (a *ImageSearch) String() string {
	return fmt.Sprintf("%s | %d items in `%s`", a.Name, len(a.Targets), a.SearchArea.Name)
}

func (a *ImageSearch) Icon() fyne.Resource {
	return assets.ImageSearchIcon
}
