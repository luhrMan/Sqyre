package actions

import (
	"fmt"
	"slices"
)

type ImageSearch struct {
	Targets                []string   `mapstructure:"targets"`
	SearchArea             SearchArea `mapstructure:"searcharea"`
	RowSplit               int        `mapstructure:"rowsplit"`
	ColSplit               int        `mapstructure:"colsplit"`
	Tolerance              float32    `mapstructure:"tolerance"`
	Blur                   int        `mapstructure:"blur"`
	OutputXVariable        string     `mapstructure:"outputxvariable"`
	OutputYVariable        string     `mapstructure:"outputyvariable"`
	WaitTilFound           bool       `mapstructure:"waittilfound"`
	WaitTilFoundSeconds    int        `mapstructure:"waittilfoundseconds"`
	WaitTilFoundIntervalMs int        `mapstructure:"waittilfoundintervalms"`
	*AdvancedAction        `yaml:",inline" mapstructure:",squash"`
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
		OutputXVariable: "foundX",
		OutputYVariable: "foundY",
	}
}

func (a *ImageSearch) String() string           { return stringifyParams(a.parameters()) }
func (a *ImageSearch) Parameters() []ActionParam { return a.parameters() }

func (a *ImageSearch) parameters() []ActionParam {
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %d seconds or until found", a.WaitTilFoundSeconds)
	}
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Items", len(a.Targets)),
		newParam("Search Area", FormatSearchAreaLabel(a.SearchArea)),
		newParam("Wait", mode),
		newParam("Tolerance", a.Tolerance),
		newParam("Blur", a.Blur),
	}
}
