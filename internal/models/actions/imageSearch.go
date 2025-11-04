package actions

import (
	"Squire/internal/models/coordinates"
	"fmt"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type ImageSearch struct {
	Targets         []string
	SearchArea      coordinates.SearchArea
	RowSplit        int
	ColSplit        int
	Tolerance       float32
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewImageSearch(name string, subActions []ActionInterface, targets []string, searchbox coordinates.SearchArea, rs, cs int, tol float32) *ImageSearch {
	slices.Sort(targets)
	return &ImageSearch{
		AdvancedAction: newAdvancedAction(name, "imagesearch", subActions),
		Targets:        targets,
		SearchArea:     searchbox,
		RowSplit:       rs,
		ColSplit:       cs,
		Tolerance:      tol,
	}
}
func (a *ImageSearch) String() string {
	return fmt.Sprintf("%s | %d items in `%s`", a.Name, len(a.Targets), a.SearchArea.Name)
}

func (a *ImageSearch) Icon() fyne.Resource {
	return theme.DesktopIcon()
}
