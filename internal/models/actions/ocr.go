package actions

import (
	"fmt"

	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

// Ocr is a leaf (basic) action: it captures a region, runs OCR, and writes the
// recognized text and match coordinates to variables. It no longer branches on
// whether the target text was found — use a Conditional action (e.g. "contains")
// on the output variable for true/false branching.
type Ocr struct {
	Name            string
	Target          string
	SearchArea      CoordinateRef `mapstructure:"searcharea"`
	OutputVariable  string
	OutputXVariable string `mapstructure:"outputxvariable"` // Variable name to store X coordinate (center of search area when found)
	OutputYVariable string `mapstructure:"outputyvariable"` // Variable name to store Y coordinate (center of search area when found)
	// Preprocessing: Blur 1-30 (odd), MinThreshold 0-255 (0=off unless Otsu), Resize 1.0-10.0, Grayscale
	Blur                   int
	MinThreshold           int
	Resize                 float64
	Grayscale              bool
	ThresholdOtsu          bool `mapstructure:"thresholdotsu"`          // Auto threshold via Otsu's method
	ThresholdInvert        bool `mapstructure:"thresholdinvert"`        // Invert binarization (light text on dark background)
	WaitTilFound           bool `mapstructure:"waittilfound"`           // If true, retry until target text found or timeout
	WaitTilFoundSeconds    int  `mapstructure:"waittilfoundseconds"`    // Max seconds to keep trying when WaitTilFound (then continue without match)
	WaitTilFoundIntervalMs int  `mapstructure:"waittilfoundintervalms"` // Milliseconds between retries when WaitTilFound (0 = default 500ms)
	*BaseAction            `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, target string, searchbox CoordinateRef) *Ocr {
	return &Ocr{
		BaseAction:      newBaseAction("ocr"),
		Name:            name,
		Target:          target,
		SearchArea:      searchbox,
		OutputVariable:  "",
		OutputXVariable: "foundX",
		OutputYVariable: "foundY",
		Blur:            1,
		MinThreshold:    0,
		Resize:          1.0,
		Grayscale:       true,
		ThresholdOtsu:   false,
		ThresholdInvert: false,
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
		newParam("Search Area", a.SearchArea.DisplayLabel()),
		newParam("Wait", mode),
	}
}

func (a *Ocr) Icon() fyne.Resource {
	return assets.TextSearchIcon
}

func (a *Ocr) VariableBindings() []VariableBinding {
	var out []VariableBinding
	if a.OutputVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputVariable, Role: "output"})
	}
	if a.OutputXVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputXVariable, Role: "output_x"})
	}
	if a.OutputYVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputYVariable, Role: "output_y"})
	}
	return out
}
