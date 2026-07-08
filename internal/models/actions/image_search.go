package actions

import (
	"slices"

)

type ImageSearch struct {
	Targets            []string      `mapstructure:"targets"`
	SearchArea         CoordinateRef `mapstructure:"searcharea"`
	RowSplit           int           `mapstructure:"rowsplit"`
	ColSplit           int           `mapstructure:"colsplit"`
	Tolerance          float32       `mapstructure:"tolerance"`
	Blur               int           `mapstructure:"blur"`
	WaitTilFoundConfig `yaml:",inline" mapstructure:",squash"`
	CoordinateOutputs  `yaml:",inline" mapstructure:",squash"`
	RunBranchOnNoFind  bool `mapstructure:"runbranchonnofind"` // If true, run sub-actions once when no targets are found
	*AdvancedAction    `yaml:",inline" mapstructure:",squash"`
}

func NewImageSearch(name string, subActions []ActionInterface, targets []string, searchbox CoordinateRef, rs, cs int, tol float32, blur int) *ImageSearch {
	slices.Sort(targets)
	return &ImageSearch{
		AdvancedAction:  newAdvancedAction(name, "imagesearch", subActions),
		Targets:         targets,
		SearchArea:      searchbox,
		RowSplit:        rs,
		ColSplit:        cs,
		Tolerance:       tol,
		Blur:            blur,
		CoordinateOutputs: CoordinateOutputs{
			OutputXVariable: "foundX",
			OutputYVariable: "foundY",
		},
	}
}
func (a *ImageSearch) String() string {
	return stringifyParams(a.Params())
}

func (a *ImageSearch) Params() []Param {
	mode := a.WaitTilFoundConfig.DisplayWaitMode("instant")
	params := []Param{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Items", len(a.Targets)),
		newParam("Search Area", a.SearchArea.DisplayLabel()),
		newExtraParam("Wait", mode),
		newExtraParam("Tolerance", a.Tolerance),
		newExtraParam("Blur", a.Blur),
	}
	if a.RunBranchOnNoFind {
		params = append(params, newExtraParam("Run on no find", "yes"))
	}
	return params
}

func (a *ImageSearch) VariableBindings() []VariableBinding {
	return a.CoordinateOutputs.VariableBindings()
}
