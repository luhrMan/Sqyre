package actions

import (
	"fmt"
	"slices"

	"Sqyre/internal/assets"
	"Sqyre/internal/config"

	"fyne.io/fyne/v2"
)

type ImageSearch struct {
	Targets                []string   `mapstructure:"targets"`
	SearchArea             SearchArea `mapstructure:"searcharea"`
	RowSplit               int        `mapstructure:"rowsplit"`
	ColSplit               int        `mapstructure:"colsplit"`
	Tolerance              float32    `mapstructure:"tolerance"`
	Blur                   int        `mapstructure:"blur"`
	OutputXVariable        string     `mapstructure:"outputxvariable"`        // Variable name to store X coordinate
	OutputYVariable        string     `mapstructure:"outputyvariable"`        // Variable name to store Y coordinate
	WaitTilFound           bool       `mapstructure:"waittilfound"`           // If true, retry until found or timeout
	WaitTilFoundSeconds    int        `mapstructure:"waittilfoundseconds"`    // Max seconds to keep trying when WaitTilFound (then continue without match)
	WaitTilFoundIntervalMs int        `mapstructure:"waittilfoundintervalms"` // Milliseconds between retries when WaitTilFound (0 = default 100ms)
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
func (a *ImageSearch) String() string {
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %d seconds or until found", a.WaitTilFoundSeconds)
	}
	return fmt.Sprintf("Name: %s %s %d items %s Search Area:%s %s Wait: %s", a.Name, config.DescriptionDelimiter, len(a.Targets), config.DescriptionDelimiter, a.SearchArea.Name, config.DescriptionDelimiter, mode)
}

func (a *ImageSearch) Icon() fyne.Resource {
	return assets.ImageSearchIcon
}
