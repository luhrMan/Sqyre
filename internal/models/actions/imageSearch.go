package actions

import (
	"fmt"
	"slices"

	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

type ImageSearch struct {
	Targets                []string   `mapstructure:"targets"`
	SearchArea             CoordinateRef `mapstructure:"searcharea"`
	RowSplit               int        `mapstructure:"rowsplit"`
	ColSplit               int        `mapstructure:"colsplit"`
	Tolerance              float32    `mapstructure:"tolerance"`
	Blur                   int        `mapstructure:"blur"`
	OutputXVariable        string     `mapstructure:"outputxvariable"`        // Variable name to store X coordinate
	OutputYVariable        string     `mapstructure:"outputyvariable"`        // Variable name to store Y coordinate
	WaitTilFound           bool       `mapstructure:"waittilfound"`           // If true, retry until found or timeout
	WaitTilFoundSeconds    int        `mapstructure:"waittilfoundseconds"`    // Max seconds to keep trying when WaitTilFound (then continue without match)
	WaitTilFoundIntervalMs int        `mapstructure:"waittilfoundintervalms"` // Milliseconds between retries when WaitTilFound (0 = default 100ms)
	RunBranchOnNoFind      bool       `mapstructure:"runbranchonnofind"`      // If true, run sub-actions once when no targets are found
	*AdvancedAction        `yaml:",inline" mapstructure:",squash"`
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
		OutputXVariable: "foundX",
		OutputYVariable: "foundY",
	}
}
func (a *ImageSearch) String() string {
	return stringifyParams(a.parameters())
}

func (a *ImageSearch) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *ImageSearch) parameters() []actionParam {
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %d seconds or until found", a.WaitTilFoundSeconds)
	}
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Items", len(a.Targets)),
		newParam("Search Area", a.SearchArea.DisplayLabel()),
		newParam("Wait", mode),
		newParam("Tolerance", a.Tolerance),
		newParam("Blur", a.Blur),
	}
	if a.RunBranchOnNoFind {
		params = append(params, newParam("Run on no find", "yes"))
	}
	return params
}

func (a *ImageSearch) Icon() fyne.Resource {
	return assets.ImageSearchIcon
}

func (a *ImageSearch) VariableBindings() []VariableBinding {
	var out []VariableBinding
	if a.OutputXVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputXVariable, Role: "output_x"})
	}
	if a.OutputYVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputYVariable, Role: "output_y"})
	}
	return out
}
