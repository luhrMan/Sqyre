package actions

import (
	"fmt"

	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

type Ocr struct {
	Target          string
	SearchArea      SearchArea
	OutputVariable  string
	OutputXVariable string `mapstructure:"outputxvariable"` // Variable name to store X coordinate (center of search area when found)
	OutputYVariable string `mapstructure:"outputyvariable"` // Variable name to store Y coordinate (center of search area when found)
	// Preprocessing: Blur 0-30 (0=off), MinThreshold 0-255 (0=off), Resize 1.0-10.0, Grayscale
	Blur                   int
	MinThreshold           int
	Resize                 float64
	Grayscale              bool
	WaitTilFound           bool `mapstructure:"waittilfound"`           // If true, retry until target text found or timeout
	WaitTilFoundSeconds    int  `mapstructure:"waittilfoundseconds"`    // Max seconds to keep trying when WaitTilFound (then continue without match)
	WaitTilFoundIntervalMs int  `mapstructure:"waittilfoundintervalms"` // Milliseconds between retries when WaitTilFound (0 = default 500ms)
	*AdvancedAction        `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox SearchArea) *Ocr {
	return &Ocr{
		AdvancedAction:  newAdvancedAction(name, "ocr", subActions),
		Target:          target,
		SearchArea:      searchbox,
		OutputVariable:  "",
		OutputXVariable: "foundX",
		OutputYVariable: "foundY",
		Blur:            3,
		MinThreshold:    50,
		Resize:          1.0,
		Grayscale:       true,
	}
}

func (a *Ocr) String() string {
	return stringifyParams(a.parameters())
}

func (a *Ocr) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Ocr) parameters() []actionParam {
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %d seconds or until found", a.WaitTilFoundSeconds)
	}
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Target Text", a.Target),
		newParam("Search Area", formatSearchAreaLabel(a.SearchArea)),
		newParam("Wait", mode),
	}
}

func (a *Ocr) Icon() fyne.Resource {
	return assets.TextSearchIcon
}
