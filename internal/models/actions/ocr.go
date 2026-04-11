package actions

import "fmt"

type Ocr struct {
	Target          string
	SearchArea      SearchArea
	OutputVariable  string
	OutputXVariable string `mapstructure:"outputxvariable"`
	OutputYVariable string `mapstructure:"outputyvariable"`
	Blur                   int
	MinThreshold           int
	Resize                 float64
	Grayscale              bool
	WaitTilFound           bool `mapstructure:"waittilfound"`
	WaitTilFoundSeconds    int  `mapstructure:"waittilfoundseconds"`
	WaitTilFoundIntervalMs int  `mapstructure:"waittilfoundintervalms"`
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

func (a *Ocr) String() string           { return stringifyParams(a.parameters()) }
func (a *Ocr) Parameters() []ActionParam { return a.parameters() }

func (a *Ocr) parameters() []ActionParam {
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %d seconds or until found", a.WaitTilFoundSeconds)
	}
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Target Text", a.Target),
		newParam("Search Area", FormatSearchAreaLabel(a.SearchArea)),
		newParam("Wait", mode),
	}
}
