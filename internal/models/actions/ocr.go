package actions

import (
	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

// Ocr is a leaf (basic) action: it captures a region, runs OCR, and writes the
// recognized text and match coordinates to variables. It no longer branches on
// whether the target text was found — use a Conditional action (e.g. "contains")
// on the output variable for true/false branching.
type Ocr struct {
	Name           string
	Target         string
	SearchArea     CoordinateRef `mapstructure:"searcharea"`
	OutputVariable string
	CoordinateOutputs `yaml:",inline" mapstructure:",squash"`
	// Preprocessing: Blur 1-30 (odd), MinThreshold 0-255 (0=off unless Otsu), Resize 1.0-10.0, Grayscale
	Blur            int
	MinThreshold    int
	Resize          float64
	Grayscale       bool
	ThresholdOtsu   bool `mapstructure:"thresholdotsu"`   // Auto threshold via Otsu's method
	ThresholdInvert bool `mapstructure:"thresholdinvert"` // Invert binarization (light text on dark background)
	WaitTilFoundConfig `yaml:",inline" mapstructure:",squash"`
	*BaseAction        `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, target string, searchbox CoordinateRef) *Ocr {
	return &Ocr{
		BaseAction:      newBaseAction("ocr"),
		Name:            name,
		Target:          target,
		SearchArea:      searchbox,
		OutputVariable: "",
		CoordinateOutputs: CoordinateOutputs{
			OutputXVariable: "foundX",
			OutputYVariable: "foundY",
		},
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
	mode := a.WaitTilFoundConfig.DisplayWaitMode("instant")
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
	out := a.CoordinateOutputs.VariableBindings()
	if a.OutputVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputVariable, Role: "output"})
	}
	return out
}
